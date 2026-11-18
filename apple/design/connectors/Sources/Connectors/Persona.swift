import CooeeCore
@testable import CooeeUI
import SwiftUI

internal struct Attached: Identifiable {
    internal let connector: Connector
    internal let account: Account

    internal var id: String { connector.name }
}

public struct PersonaConnectors: View {
    @Environment(\.look) private var look
    private let menu: Bool

    private static let attached = [
        Attached(connector: Story.linear, account: Story.linear.accounts[1]),
        Attached(connector: Story.github, account: Story.github.accounts[0]),
    ]

    public var body: some View {
        let palette = look.palette
        let coder = Fixtures.personas[1]
        ZStack(alignment: .bottomLeading) {
            VStack(spacing: 14) {
                HStack {
                    RoundButton(symbol: "xmark", ascii: "x")
                    Spacer()
                    Text(coder.api.name).ui(17, .semibold, look)
                    Spacer()
                    RoundButton(symbol: "checkmark", ascii: "ok")
                }
                .padding(.horizontal, 16).padding(.top, 14)
                Scroller {
                    VStack(alignment: .leading, spacing: 18) {
                        Group2(header: "Instructions") {
                            Line(divider: false) { Text(coder.api.instructions).ui(16, .regular, look) }
                        }
                        Group2(header: "Connectors") {
                            ForEach(Self.attached) { PickLine(pick: $0) }
                            Line(divider: false) {
                                HStack(spacing: 8) {
                                    Glyph(symbol: "plus.circle.fill", ascii: "+", size: 16)
                                    Text(look.mono ? "attach a connector" : "Attach a connector").ui(15, .regular, look)
                                }
                                .foregroundStyle(palette.link)
                            }
                        }
                        Group2 {
                            SettingRow(symbol: "folder.fill", title: "Folder", tint: palette.amber, value: "~/src/cooee")
                            SettingRow(symbol: "desktopcomputer", title: "Machine", tint: palette.cyan, value: "studio", divider: false)
                        }
                    }
                    .padding(.horizontal, 16).padding(.bottom, 20)
                }
            }
            .background(look.mono ? palette.background : palette.card.opacity(0.35))
            .foregroundStyle(palette.foreground)
            if menu {
                Dim()
                AttachMenu().padding(.leading, 32).padding(.bottom, 238)
            }
        }
        .background(Backdrop())
    }

    public init(menu: Bool = true) {
        self.menu = menu
    }
}

internal struct PickLine: View {
    @Environment(\.look) private var look
    internal let pick: Attached

    internal var body: some View {
        let palette = look.palette
        Line(inset: 62) {
            HStack(alignment: .center, spacing: 12) {
                Mark(connector: pick.connector)
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 8) {
                        Text(pick.connector.label).ui(16, .semibold, look)
                        AccountChip(name: pick.account.name)
                    }
                    if case .needsSignIn = pick.account.state {
                        Text("needs sign-in · its tools are off until then").ui(12, .medium, look)
                            .foregroundStyle(palette.amber)
                    } else {
                        ToolCounts(connector: pick.connector)
                    }
                }
                Spacer(minLength: 4)
                if case .needsSignIn = pick.account.state {
                    ActionPill(title: "Sign in", primary: true, tint: palette.amber)
                } else {
                    ActionPill(title: "Detach", tint: palette.red)
                }
            }
        }
    }
}

internal struct AccountChip: View {
    @Environment(\.look) private var look
    internal let name: String

    internal var body: some View {
        HStack(spacing: 3) {
            Text(look.mono ? "[\(name)" : name).ui(12, .medium, look)
            Glyph(symbol: "chevron.up.chevron.down", ascii: "v]", size: 9)
        }
        .foregroundStyle(look.palette.link).padding(.horizontal, look.mono ? 0 : 8).frame(height: 22)
        .background(look.mono ? Color.clear : look.palette.link.opacity(0.14), in: Capsule())
    }
}

internal struct AttachMenu: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 0) {
            Text("Signed-in accounts").ui(12, .medium, look).foregroundStyle(palette.mute)
                .padding(.horizontal, 16).padding(.top, 12).padding(.bottom, 4)
            row(Story.linear, "main", note: "")
            row(Story.linear, "work", note: "attached", done: true)
            row(Story.acme, "main", note: "not reviewed")
            Rectangle().fill(palette.line).frame(height: 0.5)
            HStack(spacing: 10) {
                Glyph(symbol: "square.grid.2x2", ascii: "+", size: 14)
                Text(look.mono ? "from the catalogue" : "From the catalogue…").ui(15, .regular, look)
            }
            .foregroundStyle(palette.link).padding(.horizontal, 16).frame(height: 46)
        }
        .frame(width: 290)
        .background(look.mono ? palette.background : palette.card, in: .rect(cornerRadius: look.mono ? 0 : 16))
        .overlay { if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) } }
        .shadow(color: .black.opacity(0.25), radius: 18, y: 6)
    }

    private func row(_ connector: Connector, _ account: String, note: String, done: Bool = false) -> some View {
        let palette = look.palette
        return HStack(spacing: 10) {
            Mark(connector: connector, size: 26)
            Text("\(connector.label) · \(account)").ui(15, .regular, look)
            Spacer()
            Text(note).ui(12, .regular, look).foregroundStyle(note == "not reviewed" ? palette.amber : palette.mute)
            if done { Glyph(symbol: "checkmark", ascii: "x", size: 13).foregroundStyle(palette.green) }
        }
        .padding(.horizontal, 16).frame(height: 46).opacity(done ? 0.55 : 1)
    }
}
