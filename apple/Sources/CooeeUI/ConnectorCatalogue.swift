import CooeeCore
import SwiftUI

internal struct ConnectorActions {
    internal var open: (String) -> Void = { _ in }
    internal var connect: (String, String) -> Void = { _, _ in }
    internal var signOut: (String, String) -> Void = { _, _ in }
    internal var remove: (String) -> Void = { _ in }
}

internal struct CatalogueGroups: View {
    @Environment(\.look) private var look
    internal let list: ConnectorList
    internal var selected: String?
    internal var actions = ConnectorActions()

    internal var body: some View {
        let palette = look.palette
        Group2(header: "Connected") {
            ForEach(list.connectors) { connector in
                ConnectedLine(
                    connector: connector, selected: connector.name == selected,
                    last: connector.id == list.connectors.last?.id, actions: actions)
            }
        }
        Group2(header: "Add from the catalogue") {
            ForEach(list.addable) { entry in
                Button {
                    actions.open(entry.name)
                } label: {
                    CatalogueLine(connector: entry)
                }.buttonStyle(.plain)
            }
            Line(divider: false) {
                HStack(spacing: 8) {
                    Glyph(symbol: "link.badge.plus", ascii: "+", size: 14)
                    Text(look.quiet ? "add by url" : "Add by URL…").ui(15, .regular, look)
                }
                .foregroundStyle(palette.link)
                Text("Not reviewed: nobody checked who runs it. Every tool asks until you mark it a read.")
                    .ui(12, .regular, look).foregroundStyle(palette.mute)
            }
        }
    }
}

internal struct ConnectedLine: View {
    @Environment(\.look) private var look
    internal let connector: Connector
    internal var selected = false
    internal var last = false
    internal var actions = ConnectorActions()

    internal var body: some View {
        Line(divider: !last, inset: 62) {
            HStack(alignment: .top, spacing: 12) {
                ConnectorMark(connector: connector)
                VStack(alignment: .leading, spacing: 4) {
                    Text(connector.label).ui(16, .semibold, look)
                    Provenance(connector: connector)
                    if connector.accounts.isEmpty {
                        Text("no sign-in · its own variables on the Mac").ui(12, .regular, look)
                            .foregroundStyle(look.palette.mute)
                    }
                    ForEach(connector.accounts) { account in
                        HStack(spacing: 6) {
                            AccountWord(account: account)
                            Text(
                                "· \(account.personas.isEmpty ? "no persona" : account.personas.joined(separator: ", "))"
                            )
                            .ui(12, .regular, look).foregroundStyle(look.palette.mute).lineLimit(1)
                        }
                    }
                }
                Spacer(minLength: 4)
                trailing
            }
        }
        .background(selected ? look.palette.link.opacity(0.14) : .clear)
        .contentShape(.rect).onTapGesture { actions.open(connector.name) }
    }

    @ViewBuilder private var trailing: some View {
        if let account = connector.accounts.first(where: { $0.state == .needsSignIn }) {
            Button {
                actions.connect(connector.name, account.name)
            } label: {
                ActionPill(title: "Sign in", primary: true, tint: look.palette.amber)
            }
            .buttonStyle(.plain)
        } else {
            Chevron().padding(.top, 10)
        }
    }
}

internal struct CatalogueLine: View {
    @Environment(\.look) private var look
    internal let connector: Connector

    internal var body: some View {
        Line(inset: 62) {
            HStack(alignment: .top, spacing: 12) {
                ConnectorMark(connector: connector)
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 6) {
                        Text(connector.label).ui(16, .semibold, look)
                        Text(connector.host.isEmpty ? "built in · read only" : connector.host)
                            .ui(12, .regular, look).foregroundStyle(look.palette.mute)
                    }
                    Text(connector.leaves).ui(12, .regular, look).foregroundStyle(look.palette.mute)
                        .fixedSize(horizontal: false, vertical: true)
                    ToolCounts(tools: connector.tools)
                }
                Spacer(minLength: 4)
                Chevron().padding(.top, 10)
            }
        }
        .contentShape(.rect)
    }
}
