import CooeeCore
import SwiftUI

internal struct EntryBody: View {
    @Environment(\.look) private var look
    internal let connector: Connector
    internal var actions = ConnectorActions()

    internal var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            HStack(spacing: 14) {
                ConnectorMark(connector: connector, size: 56)
                VStack(alignment: .leading, spacing: 4) {
                    Text(look.label(.section, connector.label)).ui(24, .bold, look)
                    Provenance(connector: connector)
                    Text("pinned to \(connector.host) · checked \(connector.checkedDay)").ui(12, .regular, look)
                        .foregroundStyle(look.palette.mute)
                }
            }
            if connector.accounts.isEmpty {
                if connector.auth != "none" { connect }
            } else {
                AccountsGroup(connector: connector, actions: actions)
            }
            LeavesGroup(connector: connector)
            ToolsGroup(tools: connector.tools)
            if !connector.accounts.isEmpty { remove }
        }
    }

    private var connect: some View {
        VStack(alignment: .leading, spacing: 8) {
            Button {
                actions.connect(connector.name, connector.nextAccount)
            } label: {
                PrimaryLabel(title: "Connect", symbol: "person.badge.key.fill")
            }
            .buttonStyle(.plain)
            Text("Opens \(connector.label)'s own sign-in page. Your Mac keeps the key; no persona sees it.")
                .ui(12, .regular, look).foregroundStyle(look.palette.mute)
        }
    }

    private var remove: some View {
        Group2 {
            Button {
                actions.remove(connector.name)
            } label: {
                Line(divider: false) {
                    Text(look.label(.chip, "Remove \(connector.label)")).ui(15, .regular, look)
                        .foregroundStyle(look.palette.red)
                    Text("Signs every account out. Personas keep their picks, which read “not connected”.")
                        .ui(12, .regular, look).foregroundStyle(look.palette.mute)
                }
                .contentShape(.rect)
            }
            .buttonStyle(.plain)
        }
    }
}

internal struct LeavesGroup: View {
    @Environment(\.look) private var look
    internal let connector: Connector

    internal var body: some View {
        let site = connector.host.split(separator: ".").suffix(2).joined(separator: ".")
        Group2(header: "What leaves this Mac") {
            row("arrow.up.forward", "\(connector.leaves.dropLast()), at \(connector.host).")
            row("network", "Signing in tells \(site) this Mac's tailnet name.")
            row("text.bubble", "What \(connector.label) returns is read by the persona's model.", last: true)
        }
    }

    private func row(_ symbol: String, _ text: String, last: Bool = false) -> some View {
        Line(divider: !last, inset: 44) {
            HStack(alignment: .top, spacing: 12) {
                Glyph(symbol: symbol, ascii: "-", size: 14).foregroundStyle(look.palette.mute).frame(width: 18)
                Text(text).ui(14, .regular, look).fixedSize(horizontal: false, vertical: true)
            }
        }
    }
}

internal struct ToolsGroup: View {
    @Environment(\.look) private var look
    internal let tools: Connector.Tools

    internal var body: some View {
        let palette = look.palette
        Group2(header: "Tools") {
            row("Read · run without asking", tools.read, palette.green, last: false)
            row("Write · ask you first, in the chat", tools.write, palette.amber, last: tools.never.isEmpty)
            if !tools.never.isEmpty { row("Never · not offered", tools.never, palette.red, last: true) }
        }
    }

    private func row(_ title: String, _ names: [String], _ tint: Color, last: Bool) -> some View {
        Line(divider: !last) {
            HStack(spacing: 6) {
                Text("\(names.count)").ui(14, .bold, look).foregroundStyle(tint)
                Text(title).ui(14, .medium, look)
            }
            Text(names.joined(separator: "  ")).font(.system(size: 12, design: .monospaced))
                .foregroundStyle(look.palette.mute).fixedSize(horizontal: false, vertical: true)
        }
    }
}

internal struct AccountsGroup: View {
    @Environment(\.look) private var look
    internal let connector: Connector
    internal var actions = ConnectorActions()

    internal var body: some View {
        let palette = look.palette
        Group2(header: "Accounts") {
            ForEach(connector.accounts) { account in
                Line(inset: 14) {
                    HStack(alignment: .center) {
                        VStack(alignment: .leading, spacing: 3) {
                            AccountWord(account: account)
                            Text(detail(account)).ui(12, .regular, look).foregroundStyle(palette.mute)
                        }
                        Spacer(minLength: 6)
                        trailing(account)
                    }
                }
            }
            Button {
                actions.connect(connector.name, connector.nextAccount)
            } label: {
                Line(divider: false) {
                    HStack(spacing: 8) {
                        Glyph(symbol: "plus", ascii: "+", size: 13)
                        Text(look.quiet ? "add an account" : "Add an account…").ui(15, .regular, look)
                    }
                    .foregroundStyle(palette.link)
                }
                .contentShape(.rect)
            }
            .buttonStyle(.plain)
        }
    }

    @ViewBuilder private func trailing(_ account: Connector.Account) -> some View {
        switch account.state {
        case .signedIn:
            Button {
                actions.signOut(connector.name, account.name)
            } label: {
                ActionPill(title: "Sign out", tint: look.palette.red)
            }
            .buttonStyle(.plain)
        case .needsSignIn:
            Button {
                actions.connect(connector.name, account.name)
            } label: {
                ActionPill(title: "Sign in again", primary: true, tint: look.palette.amber)
            }
            .buttonStyle(.plain)
        case .signingIn: ActionPill(title: "Cancel")
        }
    }

    private func detail(_ account: Connector.Account) -> String {
        let used = account.personas.isEmpty ? "no persona yet" : "used by \(account.personas.joined(separator: ", "))"
        switch account.state {
        case .signedIn: return "since \(account.day) · \(used)"
        case .needsSignIn: return "\(account.why ?? "") · \(used)"
        case .signingIn: return "in the sign-in sheet"
        }
    }
}
