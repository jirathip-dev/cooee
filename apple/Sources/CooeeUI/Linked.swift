import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

private struct NoteToKey: EnvironmentKey {
    fileprivate static var defaultValue: String? { nil }
}

extension EnvironmentValues {
    internal var noteTo: String? {
        get { self[NoteToKey.self] }
        set { self[NoteToKey.self] = newValue }
    }
}

public struct LinkedActions {
    public var chat: ChatActions
    public var open: (String) -> Void
    public var theirs: () -> Void

    public init(
        chat: ChatActions = ChatActions(), open: @escaping (String) -> Void = { _ in },
        theirs: @escaping () -> Void = {}
    ) {
        self.chat = chat
        self.open = open
        self.theirs = theirs
    }
}

public struct LinkedView: View {
    @Environment(\.look) private var look
    @Environment(\.cast) private var cast
    private let exchange: Exchange
    private let items: [Item]
    private let back: Bool
    private let actions: LinkedActions

    public var body: some View {
        let pair = [exchange.asker, exchange.target].map { api in
            (cast.first { $0.id == api.id } ?? Persona(api)).with(state: .done)
        }
        let target = pair[1]
        ZStack {
            Backdrop()
            VStack(spacing: 0) {
                LinkedHeader(exchange: exchange, pair: pair, back: back, onBack: actions.chat.back)
                Scroller(end: true) {
                    VStack(alignment: .leading, spacing: 16) {
                        ForEach(items) { item in
                            ItemView(item: item).padding(.leading, Self.indented(item) ? 42 : 0)
                                .padding(.top, Self.indented(item) ? -24 : 0)
                        }
                    }
                    .padding(.horizontal, back ? 2 : 24).padding(.top, 8).padding(.bottom, 16).frame(maxWidth: 900)
                }
                .frame(maxHeight: .infinity, alignment: .top)
                .environment(\.asker, target).environment(\.askDepth, exchange.depth + 1)
                .environment(\.noteTo, target.name).environment(\.openThread) { actions.open($0) }
                footer(pair)
            }
            .padding(.horizontal, back ? 12 : 0)
        }
        .environment(\.members, pair)
        .foregroundStyle(look.palette.foreground)
    }

    private func footer(_ pair: [Persona]) -> some View {
        let (palette, asker, target) = (look.palette, pair[0], pair[1])
        return VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                HStack(spacing: 3) {
                    ForEach(0..<Exchange.depth, id: \.self) { step in
                        RoundedRectangle(cornerRadius: look.radius(3))
                            .fill(step < exchange.depth ? palette.link : palette.edge).frame(width: 18, height: 6)
                    }
                }
                Text(exchange.meter).ui(14, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
            }
            HStack(spacing: 8) {
                chatButton("\(asker.name)'s chat") { actions.open(exchange.from) }
                chatButton("\(target.name)'s chat", actions.theirs)
            }
            Composer(hint: "Note to \(target.name)", actions: actions.chat).padding(.top, 4)
        }
        .padding(.top, 12).padding(.bottom, back ? 26 : 14).padding(.horizontal, back ? 0 : 24).frame(maxWidth: 900)
        .frame(maxWidth: .infinity)
        .overlay(alignment: .top) {
            Rectangle().fill(palette.line).frame(height: 1).padding(.horizontal, back ? -12 : 0)
        }
    }

    private func chatButton(_ title: String, _ action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title).ui(15, .semibold, look).foregroundStyle(look.palette.link).lineLimit(1)
                .frame(maxWidth: .infinity).frame(height: 40).surface(look, radius: 20)
        }
        .buttonStyle(.plain)
    }

    private static func indented(_ item: Item) -> Bool {
        if case .tools = item { true } else { false }
    }

    public init(
        exchange: Exchange, items: [Item], back: Bool = true, actions: LinkedActions = LinkedActions()
    ) {
        self.exchange = exchange
        self.items = items
        self.back = back
        self.actions = actions
    }
}

internal struct LinkedHeader: View {
    @Environment(\.look) private var look
    internal let exchange: Exchange
    internal let pair: [Persona]
    internal var back = true
    internal var onBack: () -> Void = {}

    internal var body: some View {
        let tail = back ? "" : " · ask_persona"
        HStack(spacing: 10) {
            if back {
                Button(action: onBack) { RoundButton(symbol: "chevron.left", ascii: "<", size: 40) }
                    .buttonStyle(.plain).accessibilityLabel("Back")
            }
            RoomStack(members: pair, size: 30, grid: false, state: .done)
            VStack(alignment: .leading, spacing: 2) {
                Text("\(exchange.asker.name) ⇄ \(exchange.target.name)").ui(17, .semibold, look).lineLimit(1)
                    .minimumScaleFactor(0.8)
                Text("linked from \(exchange.asker.name) · \(exchange.title)\(tail)").ui(13, .regular, look)
                    .foregroundStyle(look.palette.mute).lineLimit(1)
            }
            Spacer(minLength: 4)
            RoundButton(symbol: "ellipsis", ascii: "…", size: 40)
        }
        .padding(.horizontal, back ? 2 : 14).padding(.vertical, 8)
        .overlay(alignment: .bottom) { Rectangle().fill(look.palette.line).frame(height: 1) }
    }
}

internal struct LinkedScreen: View {
    internal let store: StoreOf<ChatFeature>
    internal let back: Bool
    internal let onBack: () -> Void

    internal var body: some View {
        if let exchange = store.exchange {
            let chat = ChatActions(
                draft: Binding(get: { store.draft }, set: { store.send(.compose(.draftChanged($0))) }),
                files: store.files.map(\.name), send: { store.send(.compose(.send)) },
                attach: { store.send(.compose(.attach($0))) },
                detach: { store.send(.compose(.detach($0))) }, back: onBack
            )
            let theirs =
                exchange.chat.map { ChatFeature.Action.delegate(.open($0)) } ?? .delegate(.start(exchange.target.id))
            LinkedView(
                exchange: exchange, items: store.drawn, back: back,
                actions: LinkedActions(
                    chat: chat, open: { store.send(.delegate(.open($0))) }, theirs: { store.send(theirs) })
            )
            .task { await store.send(.task).finish() }
        }
    }
}
