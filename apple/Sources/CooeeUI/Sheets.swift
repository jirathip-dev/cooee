import CooeeCore
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
                    palette.card.opacity(look.style.surface == .glass ? 0.75 : 1),
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
    internal var icon: AppIcon?
    internal var value = ""
    internal var divider = true

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 12) {
            if let icon {
                IconImage(icon: icon, size: 30)
            } else if !look.mono {
                Image(systemName: symbol).font(.system(size: 14, weight: .semibold)).foregroundStyle(.white)
                    .frame(width: 30, height: 30).background(tint ?? palette.link, in: .rect(cornerRadius: 8))
            }
            Text(title).ui(16, .regular, look)
            Spacer()
            Text(value).ui(15, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
            Image(systemName: "chevron.right").font(.system(size: 12, weight: .semibold))
                .foregroundStyle(palette.mute)
        }
        .padding(.horizontal, 14).frame(height: 48)
        .overlay(alignment: .bottom) {
            if divider {
                Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, look.mono ? 0 : 56)
            }
        }
    }
}
