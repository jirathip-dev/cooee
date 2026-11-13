import SwiftUI

extension PanelSections {
    internal var screen: some View {
        Group2(header: "Screen") {
            page.overlay(alignment: .topTrailing) { badge(look.mono ? "live" : "● LIVE") }.padding(8)
            if content.sites > 0 {
                more("Sites · \(content.sites)").overlay(alignment: .top) { rule(true) }
            }
        }
    }

    internal var terminal: some View {
        Group2(header: "Terminal") {
            VStack(alignment: .leading, spacing: 3) {
                ForEach(content.terminal, id: \.self) { line in
                    Text(line).font(.system(size: 11, design: .monospaced)).lineLimit(1)
                }
                Rectangle().frame(width: 7, height: 13)
            }
            .foregroundStyle(Palette.farmDark.green).padding(12)
            .frame(maxWidth: .infinity, alignment: .topLeading)
            .background(.black.opacity(look.palette.dark ? 0.55 : 0.85), in: .rect(cornerRadius: look.mono ? 0 : 10))
            .overlay(alignment: .bottomTrailing) { badge(look.mono ? "open" : "⤢") }
            .padding(8)
        }
    }

    private var page: some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 8) {
            Text(content.screen).ui(10, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
                .padding(.horizontal, 10).frame(maxWidth: .infinity, alignment: .leading).frame(height: 20)
                .background(palette.edge.opacity(0.35), in: .capsule)
            bar(palette.foreground, height: 9).frame(width: 120)
            ForEach([palette.green, palette.red, palette.green], id: \.self) { dot in
                HStack(spacing: 6) {
                    Circle().fill(dot).frame(width: 7, height: 7)
                    bar(palette.edge, height: 7)
                }
            }
        }
        .padding(10).frame(maxWidth: .infinity, minHeight: 116, alignment: .topLeading)
        .background(palette.background, in: .rect(cornerRadius: look.mono ? 0 : 10))
    }

    private func bar(_ color: Color, height: CGFloat) -> some View {
        RoundedRectangle(cornerRadius: look.mono ? 0 : 2).fill(color.opacity(0.6)).frame(height: height)
    }

    private func badge(_ text: String) -> some View {
        Text(text).ui(10, .bold, look).foregroundStyle(.white).padding(.horizontal, 7).frame(
            minWidth: 24, minHeight: 20
        )
        .background(.black.opacity(0.5), in: .rect(cornerRadius: look.mono ? 0 : 6)).padding(8)
    }
}
