import CooeeCore
import SwiftUI

internal struct BlockView: View, Equatable {
    @Environment(\.look) private var look
    @Environment(\.members) private var members
    internal let block: Block

    nonisolated internal static func == (lhs: Self, rhs: Self) -> Bool { lhs.block == rhs.block }

    internal var body: some View {
        switch block {
        case .text(let text):
            Text(Self.mentioned(Self.styled(text, look), members, look.palette.link)).ui(16, .regular, look)
                .fixedSize(horizontal: false, vertical: true)
        case .heading(let level, let text): HeadingView(level: level, text: text)
        case .list(let list): ListView(list: list, depth: 0)
        case .quote(let text): QuoteView(text: text)
        case .table(let table): TableCard(table: table)
        case .code(let lang, let code): FenceView(lang: lang, code: code)
        case .rule: RuleView()
        case .image(let alt, let url): ImageBlock(alt: alt, url: url)
        }
    }

    internal static func styled(_ text: AttributedString, _ look: Look) -> AttributedString {
        var out = text
        let (palette, mono) = (look.palette, look.type == .mono)
        for run in out.runs {
            if run.inlinePresentationIntent?.contains(.code) == true {
                out[run.range].font = .system(size: mono ? 13 : 14.5, design: .monospaced)
                out[run.range].foregroundColor = mono ? palette.cyan : palette.lead
                out[run.range].backgroundColor = mono ? nil : palette.edge.opacity(palette.dark ? 0.45 : 0.4)
            }
            if run.link != nil {
                out[run.range].foregroundColor = palette.link
                out[run.range].underlineStyle = look.glass ? nil : .single
            }
        }
        return out
    }

    internal static func mentioned(
        _ text: AttributedString, _ members: [Persona], _ tint: Color, token: Color? = nil
    ) -> AttributedString {
        var text = text
        let plain = String(text.characters)
        let cast = members.map(\.api)
        let typed = token != nil && MentionDraft(plain, members: cast).match != nil
        for mention in Mentions.all(plain)
        where Mentions.member(mention.key, cast) != nil || typed && mention.range.upperBound == plain.endIndex {
            let start = text.index(
                text.startIndex,
                offsetByCharacters: plain.distance(from: plain.startIndex, to: mention.range.lowerBound))
            let range = start..<text.index(start, offsetByCharacters: plain[mention.range].count)
            text[range].foregroundColor = tint
            if let token {
                text[range].backgroundColor = token
            } else {
                text[range].inlinePresentationIntent = .stronglyEmphasized
            }
        }
        return text
    }
}
