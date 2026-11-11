import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal typealias MacSend = (MacSetupFeature.Action) -> Void

public struct MacSetup: View {
    @Environment(\.look) private var look
    private let store: StoreOf<MacSetupFeature>
    private let fixture: Bool

    public var body: some View {
        let palette = look.palette
        let step = store.step
        HStack(spacing: 0) {
            MacStepList(step: step, footer: footer, failed: store.error != nil)
            ZStack {
                Backdrop()
                content.padding(36).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    .blur(radius: step == .approve ? 8 : 0)
                if step == .approve, let request = store.request {
                    palette.background.opacity(0.5)
                    ApprovePrompt(request: request, detail: fixture ? Fixtures.macAskedFrom : nil, send: send)
                }
            }
        }
        .foregroundStyle(palette.foreground)
    }

    private var footer: String {
        store.error ?? store.items?.summary ?? "cooee and shed run as login items. Quit from the menu bar."
    }

    @ViewBuilder private var content: some View {
        switch store.step {
        case .welcome: MacWelcome(send: send)
        case .folders: MacFolders(store: store, send: send)
        case .signins: MacSignins(store: store, send: send)
        case .tailscale: MacTailscale(store: store, send: send)
        case .pairPhone, .approve: MacPairPhone(store: store, send: send)
        }
    }

    public init(step: MacStep) {
        store = Store(initialState: Fixtures.macSetup(step)) { EmptyReducer() }
        fixture = true
    }

    public init(store: StoreOf<MacSetupFeature>) {
        self.store = store
        fixture = false
    }

    private func send(_ action: MacSetupFeature.Action) { store.send(action) }
}

internal struct MacStepEntry {
    internal let step: MacStep
    internal let title: String
    internal let icon: String
}

internal struct MacStepList: View {
    private static let steps = [
        MacStepEntry(step: .welcome, title: "Welcome", icon: "hand.wave"),
        MacStepEntry(step: .folders, title: "Folders & agents", icon: "folder"),
        MacStepEntry(step: .signins, title: "Sign-ins", icon: "person.badge.key"),
        MacStepEntry(step: .tailscale, title: "Tailscale", icon: "network"),
        MacStepEntry(step: .pairPhone, title: "Pair a phone", icon: "qrcode"),
    ]

    @Environment(\.look) private var look
    internal let step: MacStep
    internal let footer: String
    internal let failed: Bool

    internal var body: some View {
        let palette = look.palette
        let current = Self.steps.firstIndex { $0.step == (step == .approve ? .pairPhone : step) } ?? 0
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 8) {
                AppMark(size: 34)
                Text("cooee setup").ui(15, .bold, look)
            }
            .padding(.bottom, 14)
            ForEach(Array(Self.steps.enumerated()), id: \.element.step) { index, entry in
                row(index: index, current: current, title: entry.title, icon: entry.icon)
            }
            Spacer()
            Text(footer).ui(11, .regular, look).foregroundStyle(failed ? palette.red : palette.mute)
        }
        .padding(16).padding(.top, 30).frame(width: 220).background(palette.card.opacity(0.55))
    }

    private func row(index: Int, current: Int, title: String, icon: String) -> some View {
        let palette = look.palette
        return HStack(spacing: 10) {
            Image(systemName: index < current ? "checkmark.circle.fill" : icon).frame(width: 18)
                .foregroundStyle(index < current ? palette.green : (index == current ? palette.lead : palette.mute))
            Text(title).ui(13, index == current ? .semibold : .regular, look)
                .foregroundStyle(index > current ? palette.mute : palette.foreground)
        }
        .padding(.horizontal, 10).frame(height: 32).frame(maxWidth: .infinity, alignment: .leading)
        .background(index == current ? palette.link.opacity(0.16) : .clear, in: .rect(cornerRadius: 8))
    }
}

internal struct MacNav: View {
    internal let send: MacSend

    internal var body: some View {
        HStack {
            MacButton(title: "Back") { send(.back) }
            Spacer()
            MacButton(title: "Continue", primary: true) { send(.next) }
        }
    }
}

internal struct ApprovePrompt: View {
    @Environment(\.look) private var look
    internal let request: PairRequest
    internal let detail: String?
    internal let send: MacSend

    internal var body: some View {
        let palette = look.palette
        VStack(spacing: 12) {
            Image(systemName: "iphone.gen3.radiowaves.left.and.right").font(.system(size: 36))
                .foregroundStyle(palette.link)
            Text("\(request.label) wants to pair").ui(17, .bold, look)
            Text("Approve only if it shows the same word:").ui(13, .regular, look).foregroundStyle(palette.mute)
            Text(request.word).font(.system(size: 30, weight: .bold, design: .monospaced)).tracking(3)
                .foregroundStyle(palette.lead)
            if let detail { Text(detail).ui(12, .regular, look).foregroundStyle(palette.mute) }
            HStack(spacing: 10) {
                MacButton(title: "Decline") { send(.answer(false)) }.frame(maxWidth: .infinity)
                MacButton(title: "Approve · Touch ID", primary: true) { send(.answer(true)) }.frame(maxWidth: .infinity)
            }
            .padding(.top, 6)
        }
        .padding(24).frame(width: 380).surface(look, radius: 24)
    }
}

internal struct MacButton: View {
    @Environment(\.look) private var look
    internal let title: String
    internal var primary = false
    internal var action: () -> Void = {}

    internal var body: some View {
        let palette = look.palette
        Button(action: action) {
            Text(title).ui(13, primary ? .semibold : .regular, look)
                .foregroundStyle(primary ? (palette.dark ? Color.black : .white) : palette.foreground)
                .padding(.horizontal, 16).frame(height: 30)
                .background(primary ? palette.link : palette.edge.opacity(0.45), in: .capsule)
        }
        .buttonStyle(.plain)
    }
}

internal struct MacList<Content: View>: View {
    @Environment(\.look) private var look
    @ViewBuilder internal var content: Content

    internal var body: some View {
        VStack(spacing: 0) { content }.background(look.palette.card.opacity(0.75), in: .rect(cornerRadius: 12))
    }
}

internal struct MacRow: View {
    @Environment(\.look) private var look
    internal let icon: String
    internal let title: String
    internal let detail: String
    internal var done = false
    internal var link = false
    internal var trailing = ""
    internal var last = false
    internal var action: () -> Void = {}

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 10) {
            Image(systemName: icon).foregroundStyle(done ? palette.green : (link ? palette.link : palette.mute))
                .frame(width: 20)
            Text(title).ui(14, .regular, look).foregroundStyle(link ? palette.link : palette.foreground)
            Text(detail).ui(12, .regular, look).foregroundStyle(palette.mute)
            Spacer()
            if !trailing.isEmpty { MacButton(title: trailing, primary: trailing.hasSuffix("…"), action: action) }
        }
        .padding(.horizontal, 12).frame(height: 40)
        .contentShape(.rect).onTapGesture { if link { action() } }
        .overlay(alignment: .bottom) {
            if !last { Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, 42) }
        }
    }
}
