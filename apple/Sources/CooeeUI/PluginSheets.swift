import ComposableArchitecture
import BlocksUI
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct SheetBar: View {
    @Environment(\.look) private var look
    internal var cancel: (label: String, run: () -> Void)?
    internal let title: String
    internal let done: String
    internal var busy = false
    internal let run: () -> Void

    internal var body: some View {
        let palette = look.palette
        HStack {
            if let cancel {
                Button(cancel.label, action: cancel.run).buttonStyle(.plain).foregroundStyle(palette.link)
            }
            Spacer()
            Text(title).ui(16, .semibold, look)
            Spacer()
            Button(done, action: run).buttonStyle(.plain).foregroundStyle(palette.link).disabled(busy)
                .accessibilityIdentifier(done)
        }
        .ui(16, .regular, look).padding(.horizontal, 16).frame(height: 52)
    }
}

internal struct PluginScreenSheet: View {
    @Environment(\.look) private var look
    @Environment(\.dismiss) private var dismiss
    #if os(iOS)
        @Environment(\.horizontalSizeClass) private var size
    #else
        private let size: UserInterfaceSizeClass? = .regular
    #endif
    internal let store: StoreOf<ScreenFeature>

    internal var body: some View {
        screen.accessibilityHidden(store.lane != nil).overlay { lane }
    }

    @ViewBuilder private var lane: some View {
        if let lane = store.scope(state: \.lane, action: \.lane.presented), case .thread(let thread) = lane.target {
            LaneChatSheet(
                store: lane, head: LaneHead(thread: thread, screen: store.screen?.root ?? .null),
                back: "‹ \(store.label)"
            ) {
                store.send(.lane(.dismiss))
            } done: {
                dismiss()
            }
        }
    }

    private var screen: some View {
        let palette = look.palette
        return VStack(spacing: 0) {
            SheetBar(cancel: back, title: store.offline ? "" : store.label, done: "Done") { dismiss() }
            if let who = store.unreachable { offline(who) }
            ScrollViewReader { proxy in
                ScrollView { content }.modifier(Refreshing(refresh: wide ? nil : refresh)).onChange(
                    of: store.screen == nil
                ) { scroll(proxy) }
            }
            .overlay(alignment: .top) { if store.slow { LoadingBar() } }
            if let said = store.said { Text(said).ui(13, .regular, look).foregroundStyle(palette.mute).padding(8) }
            if let pending = store.screen?.root["pending"] {
                PendingBar(pending: pending) {
                    store.send(.act($0))
                } open: {
                    store.send(.navigate($0))
                }
            }
        }
        .background(palette.background).foregroundStyle(palette.foreground)
        .task { await store.send(.task).finish() }
    }

    private var refresh: (@MainActor () -> Void)? {
        guard store.screen?.root["refresh"]?.bool == true else { return nil }
        return { store.send(.retry) }
    }

    private var content: some View {
        let palette = look.palette
        return Group {
            if let screen = store.screen {
                BlockScreen(
                    screen.root, source: store.source, focus: store.focus, picked: wide ? store.beside : nil,
                    beside: wide ? store.besideScreen : nil, refresh: wide ? refresh : nil, pending: store.pending,
                    act: { store.send(.act($0)) }, open: { store.send(.navigate($0)) },
                    openThread: { size == .compact ? store.send(.push($0)) : store.send(.open($0)) },
                    openBeside: wide ? { store.send(.beside($0)) } : nil
                )
                .disabled(store.offline).opacity(store.offline ? 0.6 : 1)
            } else if store.offline {
                Text("Nothing kept from \(store.label) yet.").foregroundStyle(palette.mute)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading).padding(BlockScreen.inset)
    }

    private var wide: Bool { size != .compact }

    private func scroll(_ proxy: ScrollViewProxy) {
        guard let focus = store.focus, store.screen != nil else { return }
        Task { @MainActor in proxy.scrollTo(focus, anchor: .center) }
    }

    private var back: (label: String, run: () -> Void)? {
        guard !store.path.isEmpty else { return nil }
        return ("‹ \(store.screen?.root.string("back") ?? "Back")", { store.send(.navigate(nil)) })
    }

    private func offline(_ who: String) -> some View {
        HStack(spacing: 10) {
            TimelineView(.everyMinute) { context in
                Text(
                    Offline.line(
                        who: who, title: "\(store.source.name)'s \(store.title)", asOf: store.asOf, now: context.date))
            }
            .ui(13, .medium, look)
            Spacer(minLength: 4)
            Button("Retry") { store.send(.retry) }.buttonStyle(.plain).ui(13, .semibold, look)
        }
        .foregroundStyle(look.palette.amber).padding(.horizontal, 16).padding(.vertical, 10)
        .background(look.palette.amber.opacity(0.14))
    }
}

internal struct AttachScreen: View {
    @Environment(\.look) private var look
    @Environment(\.dismiss) private var dismiss
    internal let store: StoreOf<AttachFeature>

    internal var body: some View {
        let palette = look.palette
        VStack(spacing: 0) {
            SheetBar(
                cancel: ("Cancel", { dismiss() }), title: store.screen?.string("title") ?? "", done: "Attach",
                busy: store.attaching || store.screen == nil
            ) { store.send(.attach) }
            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    if let screen = store.screen { BlockScreen(screen, source: .core) } else { ProgressView() }
                    if let error = store.error { Text(error).foregroundStyle(palette.red) }
                }
                .frame(maxWidth: .infinity, alignment: .leading).padding(16)
            }
        }
        .ui(15, .regular, look).background(palette.background).foregroundStyle(palette.foreground)
        .task { await store.send(.task).finish() }
    }
}

internal struct PendingBar: View {
    @Environment(\.look) private var look
    internal let pending: JSONValue
    internal let act: (BlockAction) -> Void
    internal let open: (String) -> Void

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 12) {
            HStack(spacing: 0) {
                Text(pending.string("count") ?? "").fontWeight(.semibold).foregroundStyle(palette.lead)
                Text(" · \(pending.string("text") ?? "")")
            }
            .ui(13, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
            Spacer(minLength: 4)
            Button {
                run(pending["discard"])
            } label: {
                Text("Discard").ui(13, .regular, look).foregroundStyle(palette.mute)
            }
            .buttonStyle(.plain)
            Button {
                run(pending["review"])
            } label: {
                CardButton(title: "Review changes", primary: true, tint: palette.lead)
            }
            .buttonStyle(.plain)
        }
        .padding(.vertical, 8).padding(.leading, 16).padding(.trailing, 8)
        .surface(look, radius: 20).padding(12)
    }

    private func run(_ tap: JSONValue?) {
        if let path = tap?.string("sheet") ?? tap?.string("open") {
            open(tap?.string("focus").map { "\(path)#\($0)" } ?? path)
        } else if let action = tap?.string("action") {
            act(BlockAction(action: action, values: tap?["values"]?.fields ?? [:]))
        }
    }
}
