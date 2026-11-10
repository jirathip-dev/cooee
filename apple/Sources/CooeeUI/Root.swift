import SwiftUI

/// The app's state: the paired server (or the pairing flow), Home's data and the open chat. `new:<persona>` is a
/// chat not started yet: its first message makes the thread.
@MainActor @Observable public final class AppModel {
    public private(set) var server: Server?
    public private(set) var home: HomeModel?
    public private(set) var pairing: Pairing?
    /// iPhone: the chats pushed over Home. Mac and iPad: the last is the one open beside the list.
    public var path: [String] = []
    public var menu = false

    public init() {
        pairing = nil
        use(Keychain.load())
    }

    func use(_ s: Server?) {
        server = s
        home = s.map { HomeModel(Client($0)) }
        pairing = s == nil ? Pairing { [weak self] in self?.use($0) } : nil
        path = []
    }

    /// The server no longer knows this device (it was removed): pair again.
    func unpaired() { Keychain.clear(); use(nil) }

    func open(_ id: String) { path = [id] }
    func back() { path.removeLast() }
}

/// The root: onboarding until paired, then Home. iPhone pushes a chat over Home; Mac and iPad show the chat list as
/// a sidebar beside the open chat.
public struct AppRoot: View {
    @Environment(\.colorScheme) var scheme
    #if os(iOS)
    @Environment(\.horizontalSizeClass) var size
    #else
    let size: UserInterfaceSizeClass? = .regular
    #endif
    let app = State(initialValue: AppModel())
    public init() {}

    public var body: some View {
        let app = app.wrappedValue
        Group {
            if let home = app.home {
                content(app, home)
                    .task {
                        while !Task.isCancelled {
                            await home.load()
                            if home.unauthorized { app.unpaired(); return }
                            try? await Task.sleep(for: .seconds(5))
                        }
                    }
            } else if let pairing = app.pairing {
                PhoneOnboarding(pairing: pairing)
            }
        }
        .sheet(isPresented: Bindable(app).menu) { AvatarMenu().presentationDetents([.medium, .large]) }
        .environment(\.look, Look(palette: .farm(scheme)))
    }

    @ViewBuilder func content(_ app: AppModel, _ home: HomeModel) -> some View {
        let actions = HomeActions(menu: { app.menu = true }, new: { app.open("new:\($0)") }, open: app.open)
        if size == .compact {
            NavigationStack(path: Bindable(app).path) {
                HomeView(personas: home.pinned, rows: home.list(), actions: actions)
                    .navigationDestination(for: String.self) { id in
                        LiveChat(app: app, home: home, id: id, back: true).toolbar(.hidden)
                    }
                    .toolbar(.hidden)
            }
        } else {
            WideView(personas: home.pinned, rows: home.list(), selected: app.path.last, actions: actions) {
                if let id = app.path.last {
                    LiveChat(app: app, home: home, id: id, back: false).id(id)
                } else {
                    Text("Pick a chat, or start one with +").foregroundStyle(.secondary).frame(maxHeight: .infinity)
                }
            }
        }
    }
}

/// A chat on the server: its model runs while it shows.
struct LiveChat: View {
    let app: AppModel
    let home: HomeModel
    let id: String
    let back: Bool
    let model = State(initialValue: ChatModel?.none)

    var persona: Persona {
        if id.hasPrefix("new:"), let p = home.persona(String(id.dropFirst(4))) { return Persona(p) }
        return home.list().first { $0.0.id == id }?.1 ?? Persona(APIPersona(id: "", name: "Chat", agent: "", model: "", effort: "",
                                                                            folder: "", character: "", instructions: "", plugins: []))
    }

    var body: some View {
        let m = model.wrappedValue
        let persona = persona
        ChatView(persona: persona, items: m.map { chatItems($0.items, live: $0.live) } ?? [], working: m?.working ?? false,
                 since: m?.since ?? Date(), queued: m?.queue.map(\.text) ?? [], back: back,
                 actions: ChatActions(
                    send: { text, files in
                        if let m { Task { await m.send(text, files: files) }; return }
                        guard let p = home.persona(String(id.dropFirst(4))) else { return }
                        Task {
                            guard let new = try? await home.client.newThread(persona: p, text: text) else { return }
                            await home.load()
                            app.path = app.path.dropLast() + [new]
                        }
                    },
                    stop: { m?.stop() },
                    steer: { i in if let m, i < m.queue.count { m.steer(m.queue[i]) } },
                    unqueue: { i in if let m, i < m.queue.count { m.unqueue(m.queue[i]) } },
                    back: app.back))
        .task(id: id) {
            guard !id.hasPrefix("new:") else { return }
            let m = ChatModel(home.client, id: id)
            model.wrappedValue = m
            await m.open()
        }
    }
}
