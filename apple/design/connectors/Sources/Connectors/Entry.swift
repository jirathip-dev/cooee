import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct EntryPage: View {
    @Environment(\.look) private var look
    private let connector: Connector

    public var body: some View {
        let palette = look.palette
        VStack(spacing: 0) {
            HStack {
                Text(look.mono ? "< connectors" : "‹ Connectors").ui(16, .regular, look).foregroundStyle(palette.link)
                Spacer()
            }
            .padding(.horizontal, 16).frame(height: 44)
            Scroller {
                EntryBody(connector: connector).padding(.horizontal, 16).padding(.vertical, 8)
            }
        }
        .background(Backdrop()).foregroundStyle(palette.foreground)
    }

    public init(connector: Connector = Story.notion) {
        self.connector = connector
    }
}

internal struct EntryBody: View {
    @Environment(\.look) private var look
    internal let connector: Connector

    internal var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            HStack(spacing: 14) {
                Mark(connector: connector, size: 56)
                VStack(alignment: .leading, spacing: 4) {
                    Text(look.mono ? "# \(connector.name)" : connector.label).ui(24, .bold, look)
                    Provenance(connector: connector)
                    Text("pinned to \(connector.host) · checked \(connector.reviewed)").ui(12, .regular, look).foregroundStyle(look.palette.mute)
                }
            }
            if connector.accounts.isEmpty {
                VStack(alignment: .leading, spacing: 8) {
                    Primary(title: "Connect", symbol: "person.badge.key.fill")
                    Text("Opens \(connector.label)'s own sign-in page. Your Mac keeps the key; no persona sees it.")
                        .ui(12, .regular, look).foregroundStyle(look.palette.mute)
                }
            } else {
                AccountsGroup(connector: connector)
            }
            LeavesGroup(connector: connector)
            ToolsGroup(connector: connector)
            if !connector.accounts.isEmpty {
                Group2 {
                    Line(divider: false) {
                        Text(look.mono ? "[remove \(connector.name)]" : "Remove \(connector.label)")
                            .ui(15, .regular, look).foregroundStyle(look.palette.red)
                        Text("Signs every account out. Personas keep their picks, which read “not connected”.")
                            .ui(12, .regular, look).foregroundStyle(look.palette.mute)
                    }
                }
            }
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
    internal let connector: Connector

    internal var body: some View {
        let palette = look.palette
        Group2(header: "Tools") {
            row("Read · run without asking", connector.read, palette.green, last: false)
            row("Write · ask you first, in the chat", connector.write, palette.amber, last: connector.never.isEmpty)
            if !connector.never.isEmpty { row("Never · not offered", connector.never, palette.red, last: true) }
        }
    }

    private func row(_ title: String, _ tools: [String], _ tint: Color, last: Bool) -> some View {
        Line(divider: !last) {
            HStack(spacing: 6) {
                Text("\(tools.count)").ui(14, .bold, look).foregroundStyle(tint)
                Text(title).ui(14, .medium, look)
            }
            Text(tools.joined(separator: "  ")).font(.system(size: 12, design: .monospaced))
                .foregroundStyle(look.palette.mute).fixedSize(horizontal: false, vertical: true)
        }
    }
}

internal struct AccountsGroup: View {
    @Environment(\.look) private var look
    internal let connector: Connector

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
            Line(divider: false) {
                HStack(spacing: 8) {
                    Glyph(symbol: "plus", ascii: "+", size: 13)
                    Text(look.mono ? "add an account" : "Add an account…").ui(15, .regular, look)
                }
                .foregroundStyle(palette.link)
            }
        }
    }

    @ViewBuilder private func trailing(_ account: Account) -> some View {
        switch account.state {
        case .signedIn: ActionPill(title: "Sign out", tint: look.palette.red)
        case .needsSignIn: ActionPill(title: "Sign in again", primary: true, tint: look.palette.amber)
        case .signingIn: ActionPill(title: "Cancel")
        }
    }

    private func detail(_ account: Account) -> String {
        let used = account.personas.isEmpty ? "no persona yet" : "used by \(account.personas.joined(separator: ", "))"
        switch account.state {
        case let .signedIn(since): return "since \(since) · \(used)"
        case let .needsSignIn(why): return "\(why) · \(used)"
        case .signingIn: return "in the sign-in sheet"
        }
    }
}
