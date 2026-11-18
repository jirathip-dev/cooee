import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct BrowserPane: View {
    @Environment(\.look) private var look
    internal let name: String
    internal let sheet: BrowserSheet
    internal let strip: [ToolRow]
    internal let send: (ChatFeature.Browse) -> Void
    internal var stop: () -> Void = {}
    internal var done: () -> Void = {}
    internal var watch: @MainActor () async -> Void = {}

    internal var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            header
            if let left = sheet.left { LeaseBand(name: name, left: left, sheet: sheet, send: send) }
            BrowserPage(sheet: sheet, strip: strip, send: send)
            Filmstrip(strip: strip, replay: sheet.replay, send: send)
            if let error = sheet.error { Text(error).ui(13, .regular, look).foregroundStyle(look.palette.red) }
            Spacer(minLength: 0)
            buttons
        }
        .padding(14).background(look.palette.background)
        .task { await watch() }
    }

    private var header: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text("\(name)'s screen").ui(16, .semibold, look)
                Text(status).ui(12, .regular, look).foregroundStyle(look.palette.mute)
            }
            Spacer()
            Button("Done", action: done).ui(15, .semibold, look)
        }
    }

    private var status: String {
        if sheet.held { return "on mini · you have control" }
        return sheet.gone ?? "on mini · browsing"
    }

    @ViewBuilder private var buttons: some View {
        if sheet.held {
            TypeRow(sheet: sheet, send: send)
        } else {
            HStack(spacing: 10) {
                Button {
                    send(.take)
                } label: {
                    Label("Take control · 10 min", systemImage: "hand.raised").frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent).tint(look.palette.amber)
                Button("Stop", role: .destructive, action: stop).buttonStyle(.bordered)
            }
            .ui(15, .semibold, look)
        }
    }
}

internal struct LeaseBand: View {
    @Environment(\.look) private var look
    internal let name: String
    internal let left: Int
    internal let sheet: BrowserSheet
    internal let send: (ChatFeature.Browse) -> Void

    internal var body: some View {
        let tint = left <= 30 ? look.palette.red : look.palette.amber
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 10) {
                Text("\(left / 60):\(String(format: "%02d", left % 60)) left").ui(17, .bold, look)
                    .foregroundStyle(tint).monospacedDigit()
                Text("\(name) waits").ui(13, .regular, look).foregroundStyle(look.palette.mute)
                Spacer()
                Button("+10 min") { send(.take) }.buttonStyle(.bordered)
                Button("Hand back") { send(.handBack) }.buttonStyle(.borderedProminent).tint(tint)
            }
            TextField(
                "What you did, for \(name)", text: Binding(get: { sheet.note }, set: { send(.note($0)) })
            )
            .textFieldStyle(.roundedBorder)
        }
        .ui(13, .semibold, look).padding(10)
        .background(tint.opacity(0.12), in: .rect(cornerRadius: 12))
    }
}

internal struct BrowserPage: View {
    @Environment(\.look) private var look
    @Environment(\.frames) private var frames
    internal let sheet: BrowserSheet
    internal let strip: [ToolRow]
    internal let send: (ChatFeature.Browse) -> Void

    private var replayed: ToolRow? { strip.first { $0.frame == sheet.replay && sheet.replay != nil } }

    internal var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            address
            page.frame(maxWidth: .infinity).aspectRatio(1.6, contentMode: .fit)
                .background(look.palette.card, in: .rect(cornerRadius: 10)).clipShape(.rect(cornerRadius: 10))
            Text(replayed?.title ?? sheet.live?.title ?? "").ui(14, .semibold, look).lineLimit(2)
        }
    }

    private var address: some View {
        HStack(spacing: 6) {
            Image(systemName: "lock.fill").foregroundStyle(look.palette.mute)
            Text(sheet.live?.url ?? "").lineLimit(1).truncationMode(.middle)
            Spacer()
            Text(replayed == nil ? "live" : "replay").foregroundStyle(
                replayed == nil ? look.palette.green : look.palette.amber)
        }
        .ui(12, .regular, look).padding(.horizontal, 10).frame(height: 30)
        .background(look.palette.card, in: .capsule)
    }

    @ViewBuilder private var page: some View {
        let data = replayed?.frame.flatMap { frames.images[$0] } ?? sheet.live?.jpeg
        if let image = FrameThumb.image(data) {
            Image(decorative: image, scale: 1).resizable().scaledToFit()
                .overlay { GeometryReader { geometry in tappable(image, geometry.size) } }
                .accessibilityLabel(replayed == nil ? "Live view" : "Replayed frame")
        } else {
            Text(sheet.gone ?? "Waiting for the first frame…").ui(13, .regular, look).foregroundStyle(look.palette.mute)
        }
    }

    @ViewBuilder private func tappable(_ image: CGImage, _ size: CGSize) -> some View {
        if sheet.held && replayed == nil {
            Color.clear.contentShape(.rect).onTapGesture { tap in
                let page = CGSize(width: image.width, height: image.height)
                let point = LiveFrame.page(tap, view: size, page: page)
                send(.send(.click(left: point.x, top: point.y)))
            }
        }
    }
}

internal struct Filmstrip: View {
    @Environment(\.look) private var look
    @Environment(\.frames) private var frames
    internal let strip: [ToolRow]
    internal let replay: String?
    internal let send: (ChatFeature.Browse) -> Void

    internal var body: some View {
        HStack(spacing: 8) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 6) {
                    ForEach(strip.compactMap(\.frame), id: \.self) { name in
                        Button {
                            send(.replay(name))
                        } label: {
                            thumb(name)
                        }.buttonStyle(.plain)
                    }
                }
            }
            .defaultScrollAnchor(.trailing)
            if replay != nil {
                Button("Back to live") { send(.replay(nil)) }.ui(13, .semibold, look).buttonStyle(.bordered)
            }
        }
    }

    private func thumb(_ name: String) -> some View {
        Group {
            if let image = FrameThumb.image(frames.images[name]) {
                Image(decorative: image, scale: 1).resizable().scaledToFill()
            } else {
                look.palette.card.task { frames.load(name) }
            }
        }
        .frame(width: 64, height: 40).clipShape(.rect(cornerRadius: 6))
        .overlay {
            RoundedRectangle(cornerRadius: 6).stroke(name == replay ? look.palette.amber : .clear, lineWidth: 2)
        }
        .accessibilityLabel("Frame")
    }
}

internal struct TypeRow: View {
    @Environment(\.look) private var look
    internal let sheet: BrowserSheet
    internal let send: (ChatFeature.Browse) -> Void

    internal var body: some View {
        HStack(spacing: 8) {
            SecureField("Type into the page", text: Binding(get: { sheet.typed }, set: { send(.typed($0)) }))
                .textContentType(.password).textFieldStyle(.roundedBorder)
                .onSubmit { send(.send(.text(sheet.typed))) }
            Button("Type") { send(.send(.text(sheet.typed))) }.buttonStyle(.borderedProminent)
                .tint(look.palette.amber).disabled(sheet.typed.isEmpty)
        }
        .ui(15, .semibold, look)
    }
}

internal struct BrowserPresenting: ViewModifier {
    internal let store: StoreOf<ChatFeature>
    internal let wide: Bool

    internal func body(content: Content) -> some View {
        HStack(spacing: 0) {
            content
            if wide { pane.frame(width: 440) }
        }
        .sheet(
            isPresented: Binding(
                get: { !wide && store.browser != nil }, set: { if !$0 { store.send(.browse(.show(false))) } })
        ) { pane.presentationDetents([.large]) }
    }

    @ViewBuilder private var pane: some View {
        if let sheet = store.browser {
            BrowserPane(
                name: store.persona.name, sheet: sheet, strip: store.filmstrip, send: { store.send(.browse($0)) },
                stop: { store.send(.compose(.stop)) }, done: { store.send(.browse(.show(false))) },
                watch: { await store.send(.browse(.watch)).finish() }
            )
            .environment(\.frames, FrameImages(images: store.frames) { store.send(.frame($0)) })
        }
    }
}
