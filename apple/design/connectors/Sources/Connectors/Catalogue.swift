import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct CataloguePage: View {
    private let phone: Bool

    public var body: some View {
        SettingsPage(title: "Connectors", phone: phone) { CatalogueGroups(selected: nil) }
    }

    public init(phone: Bool = true) {
        self.phone = phone
    }
}

internal struct CatalogueGroups: View {
    @Environment(\.look) private var look
    internal let selected: String?

    internal var body: some View {
        let palette = look.palette
        Group2(header: "Connected") {
            ForEach(Story.connected) { connector in
                ConnectedLine(
                    connector: connector, selected: connector.name == selected,
                    last: connector.id == Story.connected.last?.id)
            }
        }
        Group2(header: "Add from the catalogue") {
            ForEach(Story.catalogue) { CatalogueLine(connector: $0) }
            Line(divider: false) {
                HStack(spacing: 8) {
                    Glyph(symbol: "link.badge.plus", ascii: "+", size: 14)
                    Text(look.mono ? "add by url" : "Add by URL…").ui(15, .regular, look)
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

    internal var body: some View {
        Line(divider: !last, inset: 62) {
            HStack(alignment: .top, spacing: 12) {
                Mark(connector: connector)
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
                            Text("· \(used(account))").ui(12, .regular, look).foregroundStyle(look.palette.mute)
                                .lineLimit(1)
                        }
                    }
                }
                Spacer(minLength: 4)
                trailing
            }
        }
        .background(selected ? look.palette.link.opacity(0.14) : .clear)
    }

    @ViewBuilder private var trailing: some View {
        if connector.accounts.contains(where: { if case .needsSignIn = $0.state { true } else { false } }) {
            ActionPill(title: "Sign in", primary: true, tint: look.palette.amber)
        } else {
            Chevron().padding(.top, 10)
        }
    }

    private func used(_ account: Account) -> String {
        account.personas.isEmpty ? "no persona" : account.personas.joined(separator: ", ")
    }
}

internal struct CatalogueLine: View {
    @Environment(\.look) private var look
    internal let connector: Connector

    internal var body: some View {
        Line(inset: 62) {
            HStack(alignment: .top, spacing: 12) {
                Mark(connector: connector)
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 6) {
                        Text(connector.label).ui(16, .semibold, look)
                        Text(connector.publisher == "built into cooee" ? "built in · read only" : connector.host)
                            .ui(12, .regular, look).foregroundStyle(look.palette.mute)
                    }
                    Text(connector.leaves).ui(12, .regular, look).foregroundStyle(look.palette.mute)
                        .fixedSize(horizontal: false, vertical: true)
                    ToolCounts(connector: connector)
                }
                Spacer(minLength: 4)
                Chevron().padding(.top, 10)
            }
        }
    }
}
