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
                Text(look.label(.group, header)).ui(12, .medium, look)
                    .foregroundStyle(palette.mute).padding(.leading, 16)
            }
            VStack(alignment: .leading, spacing: 0) { content }
                .chip(
                    look, fill: palette.card.opacity(look.glass ? 0.75 : 1), edge: palette.edge, radius: 16,
                    paper: palette.card)
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
    internal var warn = false
    internal var divider = true

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 12) {
            if let icon {
                IconImage(icon: icon, size: 30)
            } else if !look.textMarks {
                Image(systemName: symbol).font(.system(size: 14, weight: .semibold)).foregroundStyle(.white)
                    .frame(width: 30, height: 30).background(tint ?? palette.link, in: .rect(cornerRadius: 8))
            }
            Text(title).ui(16, .regular, look)
            Spacer()
            if warn {
                HStack(spacing: 5) {
                    Text(look.textMarks ? "*" : "●").font(.system(size: look.textMarks ? 12 : 8))
                    Text(value).ui(15, .regular, look).lineLimit(1)
                }
                .foregroundStyle(palette.amber)
            } else {
                Text(value).ui(15, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
            }
            Image(systemName: "chevron.right").font(.system(size: 12, weight: .semibold))
                .foregroundStyle(palette.mute)
        }
        .padding(.horizontal, 14).frame(height: 48)
        .overlay(alignment: .bottom) {
            if divider {
                Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, look.textMarks ? 0 : 56)
            }
        }
    }
}
