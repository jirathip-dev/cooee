import CooeeFeatures
import SwiftUI

internal struct SettingsPage<Content: View>: View {
    @Environment(\.look) private var look
    internal let title: String
    internal var phone = true
    @ViewBuilder internal var content: Content

    internal var body: some View {
        let palette = look.palette
        VStack(spacing: 0) {
            if phone {
                HStack {
                    Text(look.mono ? "< settings" : "‹ Settings").ui(16, .regular, look).foregroundStyle(palette.link)
                    Spacer()
                }
                .overlay { Text(look.mono ? "# \(title.lowercased())" : title).ui(17, .semibold, look) }
                .padding(.horizontal, 16).frame(height: 44)
            } else {
                Text(look.mono ? "# \(title.lowercased())" : title).ui(26, .bold, look)
                    .frame(maxWidth: .infinity, alignment: .leading).padding(.horizontal, 28).padding(.top, 24)
            }
            Scroller {
                VStack(alignment: .leading, spacing: 22) { content }
                    .frame(maxWidth: phone ? .infinity : 720, alignment: .leading)
                    .padding(.horizontal, phone ? 16 : 28).padding(.vertical, 14)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .background(Backdrop()).foregroundStyle(palette.foreground)
    }
}

internal struct ActionPill: View {
    @Environment(\.look) private var look
    internal let title: String
    internal var primary = false
    internal var tint: Color?

    internal var body: some View {
        let palette = look.palette
        let tint = tint ?? palette.link
        Text(look.mono ? "[\(title.lowercased())]" : title).ui(13, primary ? .semibold : .medium, look)
            .foregroundStyle(primary && !look.mono ? (palette.dark ? Color.black : .white) : tint)
            .lineLimit(1).fixedSize()
            .padding(.horizontal, look.mono ? 0 : 12).frame(height: 28)
            .background(
                look.mono ? Color.clear : (primary ? tint : tint.opacity(0.14)),
                in: Capsule()
            )
    }
}

internal struct StateWord: View {
    @Environment(\.look) private var look
    internal let text: String
    internal let tint: Color

    internal var body: some View {
        HStack(spacing: 5) {
            Text(look.mono ? "*" : "●").font(.system(size: look.mono ? 12 : 8))
            Text(text)
        }
        .ui(13, .medium, look).foregroundStyle(tint).lineLimit(1).fixedSize()
    }
}

internal struct Line<Content: View>: View {
    @Environment(\.look) private var look
    internal var divider = true
    internal var inset: CGFloat = 14
    @ViewBuilder internal var content: Content

    internal var body: some View {
        VStack(alignment: .leading, spacing: 6) { content }
            .padding(.horizontal, 14).padding(.vertical, 11)
            .frame(maxWidth: .infinity, alignment: .leading)
            .overlay(alignment: .bottom) {
                if divider {
                    Rectangle().fill(look.palette.line).frame(height: 0.5).padding(.leading, look.mono ? 0 : inset)
                }
            }
    }
}

internal struct TrendLine: View {
    internal let points: [Double]
    internal let tint: Color
    internal var fill = false

    internal var body: some View {
        GeometryReader { geometry in
            let path = Self.path(points, in: geometry.size)
            ZStack {
                if fill {
                    Self.area(path, in: geometry.size).fill(
                        LinearGradient(colors: [tint.opacity(0.28), .clear], startPoint: .top, endPoint: .bottom))
                }
                path.stroke(tint, style: StrokeStyle(lineWidth: fill ? 2 : 1.5, lineCap: .round, lineJoin: .round))
            }
        }
    }

    private static func path(_ points: [Double], in size: CGSize) -> Path {
        let low = points.min() ?? 0
        let span = max((points.max() ?? 1) - low, 0.0001)
        let step = size.width / CGFloat(max(points.count - 1, 1))
        return Path { path in
            for (index, value) in points.enumerated() {
                let point = CGPoint(
                    x: CGFloat(index) * step, y: size.height - CGFloat((value - low) / span) * size.height)
                if index == 0 { path.move(to: point) } else { path.addLine(to: point) }
            }
        }
    }

    private static func area(_ line: Path, in size: CGSize) -> Path {
        var area = line
        area.addLine(to: CGPoint(x: size.width, y: size.height))
        area.addLine(to: CGPoint(x: 0, y: size.height))
        area.closeSubpath()
        return area
    }
}

internal struct MacSettings<Page: View>: View {
    @Environment(\.look) private var look
    internal let selected: MenuFeature.Page
    @ViewBuilder internal var page: Page

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 2) {
                Text(look.mono ? "# settings" : "Settings").ui(13, .semibold, look).foregroundStyle(palette.mute)
                    .padding(.leading, 10).padding(.bottom, 6)
                ForEach(Self.rows, id: \.page) { row in
                    sideRow(row.page, row.symbol, row.title, row.color(palette))
                }
                Spacer()
            }
            .padding(.top, 48).padding(.horizontal, 10).frame(width: 230)
            .background(palette.card.opacity(look.style == .modern ? 0.45 : 0.8))
            .overlay(alignment: .trailing) { Rectangle().fill(palette.line).frame(width: 1) }
            page.frame(maxWidth: .infinity)
        }
        .foregroundStyle(palette.foreground)
    }

    private static var rows: [SideRow] {
        [
            SideRow(page: .look, symbol: "paintpalette.fill", title: "Look", tint: \.amber),
            SideRow(page: .notifications, symbol: "bell.badge.fill", title: "Notifications", tint: \.red),
            SideRow(page: .signIns, symbol: "person.badge.key.fill", title: "Sign-ins", tint: \.purple),
            SideRow(page: .devices, symbol: "iphone.gen3", title: "Devices", tint: \.link),
            SideRow(page: .terminal, symbol: "terminal.fill", title: "Terminal", tint: \.mute),
            SideRow(page: .machines, symbol: "desktopcomputer", title: "Machines", tint: \.cyan),
            SideRow(page: .plugins, symbol: "puzzlepiece.extension.fill", title: "Plugins", tint: \.green),
        ]
    }

    private func sideRow(_ page: MenuFeature.Page, _ symbol: String, _ title: String, _ tint: Color) -> some View {
        let active = page == selected
        return HStack(spacing: 10) {
            if !look.mono {
                Image(systemName: symbol).font(.system(size: 11, weight: .semibold)).foregroundStyle(.white)
                    .frame(width: 22, height: 22).background(tint, in: .rect(cornerRadius: 6))
            }
            Text(look.mono && active ? "> \(title.lowercased())" : title).ui(14, active ? .semibold : .regular, look)
            Spacer()
        }
        .padding(.horizontal, 10).frame(height: 34)
        .background(active ? look.palette.link.opacity(0.18) : .clear, in: .rect(cornerRadius: look.mono ? 0 : 8))
    }
}

internal struct SideRow {
    internal let page: MenuFeature.Page
    internal let symbol: String
    internal let title: String
    internal let tint: KeyPath<Palette, Color>

    internal func color(_ palette: Palette) -> Color { palette[keyPath: tint] }
}
