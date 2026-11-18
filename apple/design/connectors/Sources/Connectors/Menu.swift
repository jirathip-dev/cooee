import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct SettingsMenu: View {
    @Environment(\.look) private var look

    public var body: some View {
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
                    SettingRow(symbol: "bell.badge.fill", title: "Notifications", tint: palette.red, value: "Asks, done")
                    SettingRow(symbol: "person.badge.key.fill", title: "Sign-ins", tint: palette.purple, value: "2")
                    SettingRow(symbol: "iphone.gen3", title: "Devices", tint: palette.link, value: "3")
                    SettingRow(symbol: "terminal.fill", title: "Terminal", tint: palette.mute, value: "mini", divider: false)
                }
                Group2 {
                    SettingRow(symbol: "desktopcomputer", title: "Machines", tint: palette.cyan, value: "mini, air")
                    SettingRow(symbol: "puzzlepiece.extension.fill", title: "Plugins", tint: palette.green, value: "4")
                    ConnectorsRow()
                }
            }
            .padding(18)
            .background(look.mono ? palette.background : palette.card.opacity(0.55), in: .rect(cornerRadius: 30))
            .overlay {
                RoundedRectangle(cornerRadius: look.mono ? 0 : 30).strokeBorder(palette.edge.opacity(0.6), lineWidth: 1)
            }
            .padding(.horizontal, 12).padding(.top, 60)
            .foregroundStyle(palette.foreground)
        }
    }

    public init() {}
}

internal struct ConnectorsRow: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 12) {
            if !look.mono {
                Image(systemName: "point.3.connected.trianglepath.dotted").font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(.white).frame(width: 30, height: 30)
                    .background(palette.purple, in: .rect(cornerRadius: 8))
            }
            Text("Connectors").ui(16, .regular, look)
            Spacer()
            HStack(spacing: 5) {
                Text(look.mono ? "*" : "●").font(.system(size: look.mono ? 12 : 8)).foregroundStyle(palette.amber)
                Text("1 needs sign-in").ui(15, .regular, look).foregroundStyle(palette.amber).lineLimit(1)
            }
            Image(systemName: "chevron.right").font(.system(size: 12, weight: .semibold)).foregroundStyle(palette.mute)
        }
        .padding(.horizontal, 14).frame(height: 48)
    }
}
