import CooeeCore
@testable import CooeeUI
import SwiftUI

internal struct StatusHeader: View {
    @Environment(\.look) private var look
    internal let link: ServerLink

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 10) {
            AppMark(size: 30)
            VStack(alignment: .leading, spacing: 1) {
                Text("Cooee").ui(15, .bold, look)
                HStack(spacing: 6) {
                    Circle().fill(tone(palette)).frame(width: 8, height: 8)
                    Text(words).ui(12, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
                }
            }
            Spacer()
        }
    }

    private var words: String {
        switch link {
        case .running(let name): "\(name) · running"
        case .unreachable(let name, let seen): "\(name) · not reachable since \(seen)"
        case .unpaired: "Not paired to a server"
        }
    }

    private func tone(_ palette: Palette) -> Color {
        switch link {
        case .running: palette.green
        case .unreachable: palette.red
        case .unpaired: palette.mute
        }
    }
}

internal struct SectionTitle: View {
    @Environment(\.look) private var look
    internal let text: String
    internal let count: Int
    internal let tint: Color

    internal var body: some View {
        HStack(spacing: 6) {
            Text(look.label(.section, text.uppercased())).ui(11, .bold, look).tracking(0.6).foregroundStyle(tint)
            if count > 0 { Badge(count: count, tint: tint) }
            Spacer()
        }
        .padding(.horizontal, 10)
    }
}

internal struct LiveLine: View {
    @Environment(\.look) private var look
    internal let row: LiveRow

    internal var body: some View {
        let palette = look.palette
        let tint = row.kind == .ask ? palette.red : palette.cyan
        HStack(spacing: 10) {
            FigureView(persona: row.persona, size: 28)
            VStack(alignment: .leading, spacing: 1) {
                HStack(spacing: 6) {
                    Text(row.title).ui(13, .semibold, look).lineLimit(1)
                    if let source = row.source { SourceTag(text: source) }
                }
                Text(row.detail).ui(12, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
            }
            Spacer(minLength: 4)
            Text(row.trailing).ui(12, .semibold, look).monospacedDigit().foregroundStyle(tint)
            Image(systemName: "chevron.right").font(.system(size: 10, weight: .bold)).foregroundStyle(palette.mute)
        }
        .padding(.horizontal, 10).frame(height: 44).contentShape(.rect)
    }
}

internal struct SourceTag: View {
    @Environment(\.look) private var look
    internal let text: String

    internal var body: some View {
        Text(text).ui(10, .semibold, look).foregroundStyle(look.palette.mute)
            .padding(.horizontal, 5).frame(height: 16)
            .chip(look, fill: look.palette.edge.opacity(0.35), edge: look.palette.edge)
    }
}

internal struct EmptyLine: View {
    @Environment(\.look) private var look
    internal let symbol: String
    internal let text: String

    internal var body: some View {
        HStack(spacing: 10) {
            Image(systemName: symbol).font(.system(size: 13, weight: .semibold)).frame(width: 28)
            Text(text).ui(13, .regular, look)
            Spacer()
        }
        .foregroundStyle(look.palette.mute).padding(.horizontal, 10).frame(height: 36)
    }
}

internal struct PairQR: View {
    @Environment(\.look) private var look
    internal let server: String
    internal var dimmed = false

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 14) {
            QRCode(text: "https://\(server).tail2b4.ts.net/pair", size: 96)
                .padding(6).background(.white, in: .rect(cornerRadius: 10))
                .opacity(dimmed ? 0.25 : 1)
            VStack(alignment: .leading, spacing: 4) {
                Text("Pair a phone").ui(13, .semibold, look)
                Text(
                    dimmed
                        ? "Pairing needs the server." : "Scan with the iPhone camera, then approve its check word here."
                )
                .ui(12, .regular, look).foregroundStyle(palette.mute).fixedSize(horizontal: false, vertical: true)
                Text("\(server).tail2b4.ts.net").ui(11, .regular, look).monospaced().foregroundStyle(palette.mute)
                    .lineLimit(1)
            }
            Spacer(minLength: 0)
        }
        .padding(10)
    }
}

internal struct MenuItem: View {
    @Environment(\.look) private var look
    internal let title: String
    internal let keys: String
    internal var strong = false

    internal var body: some View {
        HStack {
            Text(title).ui(13, strong ? .semibold : .regular, look)
            Spacer()
            Text(keys).ui(12, .regular, look).foregroundStyle(look.palette.mute)
        }
        .padding(.horizontal, 10).frame(height: 28).contentShape(.rect)
    }
}

internal struct Rule: View {
    @Environment(\.look) private var look

    internal var body: some View {
        Rectangle().fill(look.palette.line).frame(height: 1).padding(.horizontal, 10)
    }
}
