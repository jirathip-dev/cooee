import AppKit
import CooeeCore
@testable import CooeeUI
import SwiftUI
import Testing

@Suite(.serialized)
internal struct ChatFormatFrames {
    private static let reply = """
        # Paragraphs keep their line breaks
        Found it. The parser joined lines, so `you:` and `HTML` ran together.
        Now a line break stays a line break, and *blank lines* still split **paragraphs**.

        ## What changed
        1. `Markdown.blocks` keeps `\\n` inside a paragraph
        2. Lists and quotes end at a blank line
           - a nested list indents one step
             - and one more for the third level
           - numbers count from the first item
        3. Tool output folds after 12 lines

        ### Checklist
        - [x] Regression test for `you:HTML`
        - [x] Fix in `CooeeCore`
        - [ ] Re-record the snapshot frames

        > Lines are never joined: “you:HTML” is two lines.
        > — #243

        | Element | Before | After | Test | Frames |
        |:--|:--|:--|:--|--:|
        | Paragraph | joined | line breaks kept | `paragraphKeepsHardBreaks` | 6 |
        | List | `- ` only | numbered, nested, tasks | `nestedListIndents` | 6 |
        | Table | — | scrolls sideways | `tableScrolls` | 3 |

        ```swift
        mutating func item(_ line: String) -> ListItem? {
            let indent = line.prefix { $0 == " " }.count / 2
            return ListItem(marker.text, depth: indent) // "- [x] a"
        }
        ```

        ```diff
        --- a/apple/Sources/CooeeCore/Item.swift
        +++ b/apple/Sources/CooeeCore/Item.swift
        @@ -105,14 +105,17 @@ public struct Markdown {
         public struct Markdown {
        -    private var bullets: [String] = []
        +    private var list: [ListItem] = []
        +    private var quote: [String] = []
             private var code: (lang: String, lines: [String])?

             private mutating func flush() {
        -            out.append(.text(paragraph.joined()))
        +            out.append(.text(paragraph.joined(separator: "\\n")))
                     paragraph = []
                 }
        +        if !quote.isEmpty { out.append(.quote(quote)) }
             }
        ```

        ```console
        \u{1B}[1mBuilding for debugging...\u{1B}[0m
        [412/412] Linking CooeeCoreTests
        Build complete! (8.31s)
        \u{1B}[2m◇\u{1B}[0m Test run started.
        \u{1B}[32m✔\u{1B}[0m Test headingsKeepTheirLevel() passed.
        \u{1B}[32m✔\u{1B}[0m Test aQuoteKeepsItsLines() passed.
        \u{1B}[32m✔\u{1B}[0m Test aTaskListReadsItsMarksOffEachItem() passed.
        \u{1B}[32m✔\u{1B}[0m Test aTableKeepsEveryColumnAndItsRightAlignment() passed.
        \u{1B}[32m✔\u{1B}[0m Test onlyHttpAndHttpsLinksStayLinks() passed.
        \u{1B}[33m⚠︎\u{1B}[0m Test tableScrolls() \u{1B}[33mskipped\u{1B}[0m: needs a 390 frame
        \u{1B}[31m✘\u{1B}[0m Test snapshotFrames() recorded an issue
        \u{1B}[31m✘\u{1B}[0m Test run with 24 tests \u{1B}[1;31mfailed\u{1B}[0m after 0.41s.
        ```

        ---

        ![Chat after the fix](https://example.com/chat.png)

        One snapshot still differs. Details in [PR #262](https://github.com/jirathip-dev/cooee/pull/262), not [here](file:///etc/hosts).
        """

    private static let height: CGFloat = 1900

    private static func page() -> some View {
        VStack(alignment: .leading, spacing: 14) {
            ItemView(item: .you(id: 1, text: "why does the chat say you:HTML?"))
            ItemView(item: .agent(id: 2, markdown: Markdown.blocks(reply)))
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 16).padding(.vertical, 14)
    }

    internal static let all: [ConnectorFrame] = ["modern", "terminal", "scene"].flatMap { style in
        [("dark", Palette.farmDark), ("light", Palette.farmLight)].map { mode, palette in
            let frame = Frame(
                name: "chat-\(style)-iphone-\(mode)", width: Frame.phoneWidth, height: height, palette: palette,
                phone: true, view: { AnyView(page()) })
            return ConnectorFrame(style: style, frame: frame.filed("chat-format"))
        }
    }

    @MainActor @Test(arguments: all)
    internal func matchesTheChatFormat(_ frame: ConnectorFrame) throws { try frame.matchesItsDesign() }

    @MainActor @Test(arguments: [true, false])
    internal func aWideTableKeepsItsNaturalWidthAndScrollsInsideA390Column(_ still: Bool) throws {
        let table = try #require(Markdown.blocks(Self.reply).first { if case .table = $0 { true } else { false } })
        let look = Look(palette: .farmDark, style: .bundled, still: still)
        let natural = NSHostingController(rootView: BlockView(block: table).fixedSize().environment(\.look, look))
        let column = NSHostingController(rootView: BlockView(block: table).environment(\.look, look))
        let size = column.sizeThatFits(in: CGSize(width: 370, height: 10_000))
        #expect(natural.sizeThatFits(in: CGSize(width: 2_000, height: 10_000)).width > 600)
        #expect(size.width == 370)
    }
}
