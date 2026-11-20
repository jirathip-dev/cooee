import CooeeCore
import SwiftUI

internal struct WebSheetFrame: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let dark = look.palette.dark
        VStack(spacing: 0) {
            HStack {
                Text("Cancel").font(.system(size: 17)).foregroundStyle(Color.blue)
                Spacer()
                HStack(spacing: 4) {
                    Image(systemName: "lock.fill").font(.system(size: 11))
                    Text("notion.com").font(.system(size: 15, weight: .semibold))
                }
                .foregroundStyle(dark ? Color.white : .black)
                Spacer()
                Image(systemName: "safari").font(.system(size: 17)).foregroundStyle(Color.blue)
            }
            .padding(.horizontal, 16).frame(height: 52).background(dark ? Color(white: 0.11) : Color(white: 0.97))
            page.frame(maxHeight: .infinity, alignment: .top)
        }
        .background(Color.white)
        .clipShape(UnevenRoundedRectangle(topLeadingRadius: 12, topTrailingRadius: 12))
        .environment(\.colorScheme, .light)
    }

    private var page: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text("Notion").font(.system(size: 26, weight: .bold)).padding(.top, 28)
            Text("Cooee wants to access your workspace").font(.system(size: 21, weight: .semibold))
            VStack(alignment: .leading, spacing: 10) {
                grant("doc.text.magnifyingglass", "Read pages and databases you share with it")
                grant("pencil", "Create and edit pages")
                grant("person.2", "See the names of people in the workspace")
            }
            HStack {
                Text("Guy's workspace").font(.system(size: 15))
                Spacer()
                Image(systemName: "chevron.down").font(.system(size: 12, weight: .semibold))
            }
            .padding(.horizontal, 14).frame(height: 44)
            .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(Color(white: 0.85)))
            Text("Allow access").font(.system(size: 16, weight: .semibold)).foregroundStyle(.white)
                .frame(maxWidth: .infinity).frame(height: 46).background(Color.black, in: .rect(cornerRadius: 8))
            Text("Redirects to mini.<tailnet>.ts.net").font(.system(size: 12)).foregroundStyle(Color(white: 0.45))
                .frame(maxWidth: .infinity)
        }
        .foregroundStyle(Color(white: 0.1)).padding(.horizontal, 24)
    }

    private func grant(_ symbol: String, _ text: String) -> some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: symbol).font(.system(size: 14)).frame(width: 20).foregroundStyle(Color(white: 0.35))
            Text(text).font(.system(size: 15))
        }
    }
}

internal struct PersonaFrame: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        var coder = Fixtures.personas[1].api
        coder.mcp = [MCPPick(server: "linear", account: "work"), MCPPick(server: "github", account: "main")]
        return ZStack(alignment: .bottomLeading) {
            VStack(spacing: 14) {
                HStack {
                    RoundButton(symbol: "xmark", ascii: "x")
                    Spacer()
                    Text(coder.name).ui(17, .semibold, look)
                    Spacer()
                    RoundButton(symbol: "checkmark", ascii: "ok")
                }
                .padding(.horizontal, 16).padding(.top, 14)
                Scroller {
                    VStack(alignment: .leading, spacing: 18) {
                        Group2(header: "Instructions") {
                            Line(divider: false) { Text(coder.instructions).ui(16, .regular, look) }
                        }
                        PersonaConnectors(draft: .constant(coder), list: Fixtures.connectors)
                        Group2 {
                            SettingRow(
                                symbol: "folder.fill", title: "Folder", tint: palette.amber, value: "~/src/cooee")
                            SettingRow(
                                symbol: "desktopcomputer", title: "Machine", tint: palette.cyan, value: "studio",
                                divider: false)
                        }
                    }
                    .padding(.horizontal, 16).padding(.bottom, 20)
                }
            }
            .background(look.outline ? palette.background : palette.card.opacity(0.35))
            .foregroundStyle(palette.foreground)
            Dim()
            AttachMenuFrame().padding(.leading, 32).padding(.bottom, 238)
        }
        .background(Backdrop())
    }
}

internal struct AttachMenuFrame: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 0) {
            Text("Signed-in accounts").ui(12, .medium, look).foregroundStyle(palette.mute)
                .padding(.horizontal, 16).padding(.top, 12).padding(.bottom, 4)
            row("linear", "main", note: "")
            row("linear", "work", note: "attached", done: true)
            row("acme", "main", note: "not reviewed")
            Rectangle().fill(palette.line).frame(height: 0.5)
            HStack(spacing: 10) {
                Glyph(symbol: "square.grid.2x2", ascii: "+", size: 14)
                Text(look.quiet ? "from the catalogue" : "From the catalogue…").ui(15, .regular, look)
            }
            .foregroundStyle(palette.link).padding(.horizontal, 16).frame(height: 46)
        }
        .frame(width: 290)
        .chip(look, fill: palette.card, edge: palette.edge, radius: look.radius(16), paper: palette.background)
        .shadow(color: .black.opacity(0.25), radius: 18, y: 6)
    }

    private func row(_ name: String, _ account: String, note: String, done: Bool = false) -> some View {
        let palette = look.palette
        let connector = Fixtures.connector(name)
        return HStack(spacing: 10) {
            ConnectorMark(connector: connector, size: 26)
            Text("\(connector.label) · \(account)").ui(15, .regular, look)
            Spacer()
            Text(note).ui(12, .regular, look).foregroundStyle(note == "not reviewed" ? palette.amber : palette.mute)
            if done { Glyph(symbol: "checkmark", ascii: "x", size: 13).foregroundStyle(palette.green) }
        }
        .padding(.horizontal, 16).frame(height: 46).opacity(done ? 0.55 : 1)
    }
}

internal struct MacFrame: View {
    @Environment(\.look) private var look

    private static let rows: [(String, String)] = [
        ("paintpalette.fill", "Look"), ("bell.badge.fill", "Notifications"), ("person.badge.key.fill", "Sign-ins"),
        ("iphone.gen3", "Devices"), ("terminal.fill", "Terminal"), ("desktopcomputer", "Machines"),
        ("puzzlepiece.extension.fill", "Plugins"), ("point.3.connected.trianglepath.dotted", "Connectors"),
    ]
    private static let tints: [KeyPath<Palette, Color>] = [
        \.amber, \.red, \.purple, \.link, \.mute, \.cyan, \.green, \.purple,
    ]

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 0) {
            sidebar
            VStack(alignment: .leading, spacing: 0) {
                Text(look.label(.section, "Connectors")).ui(26, .bold, look)
                    .padding(.horizontal, 24).padding(.top, 24).padding(.bottom, 8)
                Scroller {
                    VStack(alignment: .leading, spacing: 22) {
                        CatalogueGroups(list: Fixtures.connectors, selected: "linear")
                    }
                    .padding(.horizontal, 16).padding(.vertical, 10)
                }
            }
            .frame(width: 440)
            .overlay(alignment: .trailing) { Rectangle().fill(palette.line).frame(width: 1) }
            Scroller {
                EntryBody(connector: Fixtures.connector("linear")).frame(maxWidth: 620).padding(.horizontal, 32)
                    .padding(.vertical, 30).frame(maxWidth: .infinity)
            }
        }
        .foregroundStyle(palette.foreground)
        .background(Backdrop())
    }

    private var sidebar: some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 2) {
            Text(look.label(.section, "Settings")).ui(13, .semibold, look).foregroundStyle(palette.mute)
                .padding(.leading, 10).padding(.bottom, 6)
            ForEach(Array(zip(Self.rows, Self.tints)), id: \.0.1) { row, tint in
                let active = row.1 == "Connectors"
                HStack(spacing: 10) {
                    if !look.textMarks {
                        Image(systemName: row.0).font(.system(size: 11, weight: .semibold)).foregroundStyle(.white)
                            .frame(width: 22, height: 22).background(
                                palette[keyPath: tint], in: .rect(cornerRadius: 6))
                    }
                    Text(active ? look.label(.picked, row.1) : row.1).ui(14, active ? .semibold : .regular, look)
                    Spacer()
                    if active { Text("●").font(.system(size: 8)).foregroundStyle(palette.amber) }
                }
                .padding(.horizontal, 10).frame(height: 34)
                .background(active ? palette.link.opacity(0.18) : .clear, in: .rect(cornerRadius: look.radius(8)))
            }
            Spacer()
        }
        .padding(.top, 48).padding(.horizontal, 10).frame(width: 230)
        .background(palette.card.opacity(look.glass ? 0.45 : 0.8))
        .overlay(alignment: .trailing) { Rectangle().fill(palette.line).frame(width: 1) }
    }
}
