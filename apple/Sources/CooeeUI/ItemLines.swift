import CooeeCore
import SwiftUI

internal struct LinkLine: View {
    @Environment(\.look) private var look
    @Environment(\.openThread) private var openThread
    internal let link: ThreadLink

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 6) {
            if !link.text.isEmpty { Text(Markdown.inline(link.text)).ui(15, .regular, look) }
            Button {
                openThread(link.thread)
            } label: {
                Label(link.chip, systemImage: link.onward ? "arrow.turn.down.right" : "arrow.turn.up.left")
                    .ui(13, .medium, look).foregroundStyle(palette.link).lineLimit(1)
                    .padding(.horizontal, 10).frame(height: 30)
                    .chip(look, fill: palette.card.opacity(0.7), edge: palette.line, radius: 10)
            }
            .buttonStyle(.plain)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

extension CodeBlock {
    internal static func diff(_ code: String, _ palette: Palette) -> AttributedString {
        var out = AttributedString()
        for (index, line) in code.split(separator: "\n", omittingEmptySubsequences: false).enumerated() {
            var part = AttributedString((index == 0 ? "" : "\n") + line)
            if line.hasPrefix("+") { part.foregroundColor = palette.green }
            if line.hasPrefix("-") { part.foregroundColor = palette.red }
            out += part
        }
        return out
    }
}
