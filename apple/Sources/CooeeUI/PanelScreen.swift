import SwiftUI

extension PanelSections {
    internal var screen: some View {
        Group2(header: "Screen") {
            page.overlay(alignment: .topTrailing) { live }.padding(8)
            if content.sites > 0 {
                more("Sites · \(content.sites)").overlay(alignment: .top) { rule(true) }
            }
        }
    }

    internal var terminal: some View {
        let palette = look.palette
        return Group2(header: "Terminal") {
            VStack(alignment: .leading, spacing: 3) {
                ForEach(content.terminal, id: \.self) { line in
                    Text(line).font(.system(size: 11, design: .monospaced)).lineLimit(1)
                }
                Rectangle().frame(width: 7, height: 13)
            }
            .foregroundStyle(Palette.farmDark.green).padding(12)
            .frame(maxWidth: .infinity, alignment: .topLeading)
            .background(Color.black.opacity(palette.dark ? 0.55 : 0.85), in: .rect(cornerRadius: look.mono ? 0 : 10))
            .overlay(alignment: .bottomTrailing) { expand }
            .padding(8)
        }
    }

    private var page: some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 8) {
            Text(content.screen).ui(10, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
                .padding(.horizontal, 10).frame(maxWidth: .infinity, alignment: .leading).frame(height: 20)
                .background(palette.edge.opacity(0.35), in: .capsule)
            HStack(alignment: .top, spacing: 8) {
                VStack(spacing: 5) { ForEach(0..<5, id: \.self) { _ in bar(palette.edge, height: 6) } }.frame(width: 56)
                VStack(alignment: .leading, spacing: 6) {
                    bar(palette.foreground.opacity(0.7), height: 9).frame(width: 120)
                    ForEach(0..<3, id: \.self) { index in
                        HStack(spacing: 6) {
                            Circle().fill(index == 1 ? palette.red : palette.green).frame(width: 7, height: 7)
                            bar(palette.edge, height: 7)
                        }
                    }
                }
            }
        }
        .padding(10).frame(maxWidth: .infinity, minHeight: 116, alignment: .topLeading)
        .background(palette.background, in: .rect(cornerRadius: look.mono ? 0 : 10))
    }

    private func bar(_ color: Color, height: CGFloat) -> some View {
        RoundedRectangle(cornerRadius: look.mono ? 0 : 2).fill(color.opacity(0.6)).frame(height: height)
    }

    private var live: some View {
        HStack(spacing: 4) {
            Circle().fill(look.palette.red).frame(width: 6, height: 6)
            Text(look.mono ? "live" : "LIVE").ui(10, .bold, look)
        }
        .foregroundStyle(.white).padding(.horizontal, 7).frame(height: 18)
        .background(.black.opacity(0.5), in: .capsule).padding(8)
    }

    private var expand: some View {
        Group {
            if look.mono {
                Text("[open]").ui(11, .bold, look)
            } else {
                Image(systemName: "arrow.up.left.and.arrow.down.right").font(.system(size: 11, weight: .bold))
            }
        }
        .foregroundStyle(.white).frame(minWidth: 26, minHeight: 26)
        .background(.black.opacity(0.5), in: .rect(cornerRadius: look.mono ? 0 : 8)).padding(8)
    }
}
