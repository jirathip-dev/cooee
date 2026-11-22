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
            Text(Self.mentioned(Markdown.inline(text), members, look.palette.link)).ui(16, .regular, look)
                .fixedSize(horizontal: false, vertical: true)
        case .bullets(let bullets):
            VStack(alignment: .leading, spacing: 4) {
                ForEach(bullets, id: \.self) { bullet in
                    Text(Markdown.inline("•  " + bullet)).ui(16, .regular, look)
                }
            }
        case .code(let lang, let code):
            CodeBlock(lang: lang, code: code)
        }
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
