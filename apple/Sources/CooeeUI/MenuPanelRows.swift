import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct StatusHeader: View {
    @Environment(\.look) private var look
    internal let link: Glance.Link

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
        case .unreachable(let name, let since?): "\(name) · not reachable since \(since)"
        case .unreachable(let name, nil): "\(name) · not reachable"
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
    internal let chat: ListedChat
    internal let tint: Color

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 10) {
            FigureView(persona: chat.persona, size: 28)
            VStack(alignment: .leading, spacing: 1) {
                HStack(spacing: 6) {
                    Text(title).ui(13, .semibold, look).lineLimit(1)
                    if let source = chat.workflow?.source { SourceTag(text: source) }
                }
                Text(detail).ui(12, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
            }
            Spacer(minLength: 4)
            Text(chat.row.ago).ui(12, .semibold, look).monospacedDigit().foregroundStyle(tint)
            Image(systemName: "chevron.right").font(.system(size: 10, weight: .bold)).foregroundStyle(palette.mute)
        }
        .padding(.horizontal, 10).frame(height: 44).contentShape(.rect)
    }

    private var title: String {
        chat.workflow.map { "\($0.issue) · \($0.role)" } ?? chat.persona.name
    }

    private var detail: String {
        let row = chat.row
        if let status = row.status { return status }
        return row.state == .needs && !row.last.isEmpty ? row.last : row.title
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

internal struct MoreLine: View {
    @Environment(\.look) private var look
    internal let count: Int

    internal var body: some View {
        HStack {
            Text("+\(count) more").ui(12, .semibold, look).foregroundStyle(look.palette.link)
            Spacer()
        }
        .padding(.horizontal, 10).frame(height: 28).contentShape(.rect)
    }
}

internal struct PairQR: View {
    @Environment(\.look) private var look
    internal let url: URL
    internal let dimmed: Bool

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 14) {
            QRCode(text: url.absoluteString, size: 96)
                .padding(6).background(.white, in: .rect(cornerRadius: 10))
                .opacity(dimmed ? 0.25 : 1)
            VStack(alignment: .leading, spacing: 4) {
                Text("Pair a phone").ui(13, .semibold, look)
                Text(
                    dimmed
                        ? "Pairing needs the server." : "Scan with the iPhone camera, then approve its check word here."
                )
                .ui(12, .regular, look).foregroundStyle(palette.mute).fixedSize(horizontal: false, vertical: true)
                Text(url.host() ?? "").ui(11, .regular, look).monospaced().foregroundStyle(palette.mute).lineLimit(1)
            }
            Spacer(minLength: 0)
        }
        .padding(10)
    }
}

internal struct PanelItem: View {
    @Environment(\.look) private var look
    internal let title: String
    internal let key: Character
    internal var strong = false
    internal let action: () -> Void

    internal var body: some View {
        Button(action: action) {
            HStack {
                Text(title).ui(13, strong ? .semibold : .regular, look)
                Spacer()
                Text("⌘\(String(key).uppercased())").ui(12, .regular, look).foregroundStyle(look.palette.mute)
            }
            .padding(.horizontal, 10).frame(height: 28).contentShape(.rect)
        }
        .buttonStyle(.plain).keyboardShortcut(KeyEquivalent(key))
    }
}

internal struct PanelRule: View {
    @Environment(\.look) private var look

    internal var body: some View {
        Rectangle().fill(look.palette.line).frame(height: 1).padding(.horizontal, 10)
    }
}
