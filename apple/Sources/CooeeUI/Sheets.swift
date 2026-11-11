import CooeeFeatures
import SwiftUI

internal struct Group2<Content: View>: View {
    @Environment(\.look) private var look
    internal var header = ""
    @ViewBuilder internal var content: Content

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 6) {
            if !header.isEmpty {
                Text(look.mono ? "# \(header.lowercased())" : header.uppercased()).ui(12, .medium, look)
                    .foregroundStyle(palette.mute).padding(.leading, 16)
            }
            VStack(alignment: .leading, spacing: 0) { content }
                .background(
                    palette.card.opacity(look.style == .modern ? 0.75 : 1),
                    in: .rect(cornerRadius: look.mono ? 0 : 16)
                )
                .overlay {
                    if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) }
                }
        }
    }
}

internal struct SettingRow: View {
    @Environment(\.look) private var look
    internal let symbol: String
    internal let title: String
    internal var tint: Color?
    internal var value = ""
    internal var divider = true
    internal var edit: Binding<String>?

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 12) {
            if !look.mono {
                Image(systemName: symbol).font(.system(size: 14, weight: .semibold)).foregroundStyle(.white)
                    .frame(width: 30, height: 30).background(tint ?? palette.link, in: .rect(cornerRadius: 8))
            }
            Text(title).ui(16, .regular, look)
            Spacer()
            if let edit, !look.still {
                TextField(title, text: edit).multilineTextAlignment(.trailing).ui(15, .regular, look)
                    .foregroundStyle(palette.mute)
            } else {
                Text(value).ui(15, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
                Image(systemName: "chevron.right").font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(palette.mute)
            }
        }
        .padding(.horizontal, 14).frame(height: 48)
        .overlay(alignment: .bottom) {
            if divider {
                Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, look.mono ? 0 : 56)
            }
        }
    }
}

internal struct AvatarMenu: View {
    @Environment(\.look) private var look
    internal var name = "Guy"
    internal var place = "this iPhone · paired to mini"
    internal var values = ["Modern · Farm", "Asks, done", "Claude, Codex", "3", "mini, air · Terminal", "4"]
    internal var pick: (MenuFeature.Page) -> Void = { _ in }

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 14) {
            HStack(spacing: 12) {
                AccountAvatar(size: 48)
                VStack(alignment: .leading, spacing: 2) {
                    Text(name).ui(18, .semibold, look)
                    Text(place).ui(13, .regular, look).foregroundStyle(palette.mute)
                }
                Spacer()
            }
            .padding(.horizontal, 6)
            Group2(header: "Settings") {
                row(.look, "paintpalette.fill", "Look", palette.amber)
                row(.notifications, "bell.badge.fill", "Notifications", palette.red)
                row(.signIns, "person.badge.key.fill", "Sign-ins", palette.purple)
                row(.devices, "iphone.gen3", "Devices", palette.link, divider: false)
            }
            Group2 {
                row(.machines, "desktopcomputer", "Machines", palette.cyan)
                row(.plugins, "puzzlepiece.extension.fill", "Plugins", palette.green, divider: false)
            }
            Text(look.mono ? "-- v2: these open their screens" : "Machines, Terminal and the plugin sheets open in v2.")
                .ui(12, .regular, look).foregroundStyle(palette.mute).padding(.leading, 6)
        }
        .padding(14)
        .surface(look, radius: 30)
        .foregroundStyle(palette.foreground)
    }

    private func row(
        _ page: MenuFeature.Page,
        _ symbol: String,
        _ title: String,
        _ tint: Color,
        divider: Bool = true
    ) -> some View {
        let value = MenuFeature.Page.allCases.firstIndex(of: page).map { values[$0] } ?? ""
        return Button {
            pick(page)
        } label: {
            SettingRow(symbol: symbol, title: title, tint: tint, value: value, divider: divider).contentShape(.rect)
        }
        .buttonStyle(.plain)
    }
}
