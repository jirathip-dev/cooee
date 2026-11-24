import CooeeCore
import SwiftUI

/// The connectors design's frames (#524, apple/design/connectors), drawn from the views the app shows.
public enum ConnectorFrames {
    @MainActor
    public static func settingsRow() -> some View { SettingsMenuFrame() }

    @MainActor
    public static func catalogue() -> some View {
        SettingsPage(title: "Connectors") { CatalogueGroups(list: Fixtures.connectors) }
    }

    @MainActor
    public static func entry() -> some View { EntryFrame(connector: Fixtures.connector("notion")) }

    @MainActor
    public static func signIn() -> some View {
        ZStack(alignment: .top) {
            EntryFrame(connector: Fixtures.connector("notion").signingIn("main"))
            Dim()
            WebSheetFrame().padding(.top, 10)
        }
    }

    @MainActor
    public static func signedIn() -> some View {
        ZStack(alignment: .bottom) {
            EntryFrame(connector: Fixtures.notionSignedIn)
            Dim()
            ConnectedSheet(label: "Notion", account: "main", personas: Fixtures.personas)
        }
    }

    @MainActor
    public static func token() -> some View {
        ZStack(alignment: .bottom) {
            EntryFrame(connector: Fixtures.connector("github"))
            Dim()
            TokenSheet(
                title: "Sign in to GitHub · main",
                help:
                    "Make a fine-grained token at github.com › Settings › Developer settings, with read access to the repositories your personas use.",
                value: .constant(""))
        }
    }

    @MainActor
    public static func persona() -> some View { PersonaFrame() }

    @MainActor
    public static func mac() -> some View { MacFrame() }
}

internal struct EntryFrame: View {
    @Environment(\.look) private var look
    internal let connector: Connector

    internal var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(look.textMarks ? look.label(.back, "Connectors") : "‹ Connectors").ui(16, .regular, look)
                    .foregroundStyle(look.palette.link)
                Spacer()
            }
            .padding(.horizontal, 16).frame(height: 44)
            Scroller { EntryBody(connector: connector).padding(.horizontal, 16).padding(.vertical, 8) }
        }
        .background(Backdrop()).foregroundStyle(look.palette.foreground)
    }
}

internal struct SettingsMenuFrame: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        ZStack(alignment: .top) {
            Backdrop()
            VStack(alignment: .leading, spacing: 14) {
                HStack(spacing: 14) {
                    Text("G").font(.system(size: 22, weight: .bold)).foregroundStyle(.black)
                        .frame(width: 48, height: 48).background(palette.amber, in: Circle())
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Guy").ui(18, .semibold, look)
                        Text("this iPhone · paired to mini").ui(13, .regular, look).foregroundStyle(palette.mute)
                    }
                }
                Group2(header: "Settings") {
                    SettingRow(symbol: "paintpalette.fill", title: "Look", tint: palette.amber, value: "Modern · Farm")
                    SettingRow(
                        symbol: "bell.badge.fill", title: "Notifications", tint: palette.red, value: "Asks, done")
                    SettingRow(symbol: "person.badge.key.fill", title: "Sign-ins", tint: palette.purple, value: "2")
                    SettingRow(symbol: "iphone.gen3", title: "Devices", tint: palette.link, value: "3")
                    SettingRow(
                        symbol: "terminal.fill", title: "Terminal", tint: palette.mute, value: "mini", divider: false)
                }
                Group2 {
                    SettingRow(symbol: "desktopcomputer", title: "Machines", tint: palette.cyan, value: "mini, lab")
                    SettingRow(symbol: "puzzlepiece.extension.fill", title: "Plugins", tint: palette.green, value: "4")
                    SettingRow(
                        symbol: "point.3.connected.trianglepath.dotted", title: "Connectors", tint: palette.purple,
                        value: "1 needs sign-in", warn: true, divider: false)
                }
            }
            .padding(18)
            .background(look.outline ? palette.background : palette.card.opacity(0.55), in: .rect(cornerRadius: 30))
            .overlay {
                RoundedRectangle(cornerRadius: look.radius(30)).strokeBorder(palette.edge.opacity(0.6), lineWidth: 1)
            }
            .padding(.horizontal, 12).padding(.top, 60)
            .foregroundStyle(palette.foreground)
        }
    }
}
