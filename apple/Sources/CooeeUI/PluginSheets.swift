import ComposableArchitecture
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
    internal let store: StoreOf<PluginScreenFeature>

    internal var body: some View {
        let palette = look.palette
        VStack(spacing: 0) {
            SheetBar(cancel: back, title: store.offline ? "" : store.place.label, done: "Done") { dismiss() }
            if let who = store.unreachable { offline(who) }
            ScrollView {
                Group {
                    if let screen = store.screen {
                        BlocksView(screen.root) {
                            store.send(.act($0))
                        } open: {
                            store.send(.navigate($0))
                        } openThread: {
                            store.send(.open($0))
                        }
                        .disabled(store.offline || store.acting).opacity(store.offline ? 0.6 : 1)
                    } else if store.offline {
                        Text("Nothing kept from \(store.place.label) yet.").foregroundStyle(palette.mute)
                    } else {
                        ProgressView()
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading).padding(BlocksView.inset)
            }
            if let said = store.said { Text(said).ui(13, .regular, look).foregroundStyle(palette.mute).padding(8) }
            if let pending = store.screen?.root["pending"] {
                PendingBar(pending: pending).environment(\.act, { store.send(.act($0)) })
                    .environment(\.open, { store.send(.navigate($0)) })
            }
        }
        .background(palette.background).foregroundStyle(palette.foreground)
        .task { await store.send(.task).finish() }
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
                        who: who, title: "\(store.place.plugin)'s \(store.title)", asOf: store.asOf, now: context.date))
            }
            .ui(13, .medium, look)
            Spacer(minLength: 4)
            Button("Retry") { store.send(.retry) }.buttonStyle(.plain).ui(13, .semibold, look)
        }
        .foregroundStyle(look.palette.amber).padding(.horizontal, 16).padding(.vertical, 10)
        .background(look.palette.amber.opacity(0.14))
    }
}

internal struct AttachSheet: View {
    @Environment(\.look) private var look
    @Environment(\.dismiss) private var dismiss
    internal let store: StoreOf<AttachFeature>

    internal var body: some View {
        let (plugin, palette) = (store.plugin, look.palette)
        VStack(spacing: 0) {
            SheetBar(
                cancel: ("Cancel", { dismiss() }), title: "Attach \(plugin.name)", done: "Attach", busy: store.attaching
            ) { store.send(.attach) }
            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    Text("\(plugin.name) \(plugin.version) · \(plugin.commit.prefix(7)) · \(plugin.source)")
                        .ui(13, .regular, look).foregroundStyle(palette.mute)
                    list("To \(store.persona.name), it may", plugin.may)
                    list("Actions", plugin.permissions.actions) { plugin.permissions.destructive.contains($0) }
                    list(
                        "Its backend reaches",
                        plugin.permissions.network.isEmpty ? ["nothing"] : plugin.permissions.network)
                    if let error = store.error { Text(error).foregroundStyle(palette.red) }
                }
                .frame(maxWidth: .infinity, alignment: .leading).padding(16)
            }
        }
        .ui(15, .regular, look).background(palette.background).foregroundStyle(palette.foreground)
    }

    private func list(_ header: String, _ lines: [String], red: @escaping (String) -> Bool = { _ in false })
        -> some View
    {
        VStack(alignment: .leading, spacing: 6) {
            Text(look.mono ? "## \(header)" : header.uppercased()).ui(12, .semibold, look)
                .foregroundStyle(look.palette.mute)
            ForEach(lines, id: \.self) { line in
                Text(red(line) ? "\(line) · always confirmed first" : line)
                    .foregroundStyle(red(line) ? look.palette.red : look.palette.foreground)
            }
        }
    }
}
