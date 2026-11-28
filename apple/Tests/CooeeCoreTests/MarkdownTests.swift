import CooeeCore
import Foundation
import Testing

internal struct MarkdownTests {
    private static func item(_ markdown: String, done: Bool? = nil, children: ListBlock? = nil) -> ListItem {
        ListItem(Markdown.inline(markdown), done: done, children: children)
    }

    @Test internal func aParagraphKeepsItsLineBreaksAndABlankLineSplitsIt() {
        #expect(
            Markdown.blocks("you:\nHTML is **two** lines\\\nthree\n\nnext")
                == [.text(Markdown.inline("you:\nHTML is **two** lines\nthree")), .text("next")])
        #expect(Markdown.blocks("") == [])
    }

    @Test internal func headingsKeepTheirLevel() {
        #expect(
            Markdown.blocks("# One\n## Two `x`\n### Three")
                == [.heading(1, "One"), .heading(2, Markdown.inline("Two `x`")), .heading(3, "Three")])
    }

    @Test internal func aNumberedListCountsFromItsFirstNumberAndNestsItsChildren() {
        let nested = ListBlock([Self.item("deeper", children: ListBlock([Self.item("deepest")]))])
        let list = ListBlock(start: 3, [Self.item("**three**"), Self.item("four", children: nested)])
        #expect(Markdown.blocks("3. **three**\n4. four\n   - deeper\n     * deepest") == [.list(list)])
        #expect(
            Markdown.blocks("intro\n- a\n```sh\nls") == [
                .text("intro"), .list(ListBlock([Self.item("a")])), .code(lang: "sh", "ls"),
            ])
    }

    @Test internal func aTaskListReadsItsMarksOffEachItem() {
        let tasks = ListBlock([
            Self.item("tested", done: true), Self.item("Fixed", done: true), Self.item("recorded", done: false),
        ])
        #expect(Markdown.blocks("- [x] tested\n- [X] Fixed\n- [ ] recorded") == [.list(tasks)])
    }

    @Test internal func aQuoteKeepsItsLines() {
        #expect(Markdown.blocks("> one\n> two\n>\n> three") == [.quote("one\ntwo\nthree")])
    }

    @Test internal func aTableKeepsEveryColumnAndItsRightAlignment() {
        let table = TableBlock(
            header: ["Element", "Frames"], trailing: [1], rows: [[Markdown.inline("`code`"), "6"], ["", "3"]])
        #expect(Markdown.blocks("| Element | Frames |\n|:--|--:|\n| `code` | 6 |\n| | 3 |") == [.table(table)])
    }

    @Test internal func aFenceKeepsItsLanguageAndARuleIsARule() {
        #expect(Markdown.blocks("```swift\nlet x = 1\n```\n\n---") == [.code(lang: "swift", "let x = 1"), .rule])
    }

    @Test internal func anImageOnItsOwnLineIsAnImageOnlyFromTheWeb() throws {
        let url = try #require(URL(string: "https://example.com/a.png"))
        #expect(Markdown.blocks("![chat](https://example.com/a.png)") == [.image(alt: "chat", url: url)])
        #expect(Markdown.blocks("![chat](file:///etc/a.png)") != [.image(alt: "chat", url: url)])
        #expect(Markdown.blocks("![chat](file:///etc/a.png)").count == 1)
    }

    @Test internal func onlyHttpAndHttpsLinksStayLinks() {
        let links = Markdown.blocks("[a](https://a.dev) [b](http://b.dev) [c](file:///etc) [d](javascript:x)")
        guard case .text(let text)? = links.first else { return #expect(Bool(false)) }
        #expect(text.runs.compactMap(\.link?.absoluteString) == ["https://a.dev", "http://b.dev"])
        #expect(String(text.characters) == "a b c d")
        #expect(Markdown.inline("[c](file:///etc)").runs.allSatisfy { $0.link == nil })
    }

    @Test internal func aDiffFenceNamesItsFileCountsItsLinesAndFoldsToItsHead() {
        let lines =
            ["--- a/x.swift", "+++ b/apple/x.swift", "@@ -1,12 +1,12 @@"] + (1...12).map { "+\($0)" } + ["-old"]
        guard case .diff(let diff) = Fence(lang: "diff", lines.joined(separator: "\n")) else {
            return #expect(Bool(false))
        }
        #expect(diff.file == "apple/x.swift" && diff.added == 12 && diff.removed == 1 && diff.lines.count == 14)
        let head = Folded.head(diff.lines)
        #expect(head.lines == Array(diff.lines.prefix(8)) && head.hidden == 6)
        #expect(Folded.head(Array(diff.lines.prefix(12))).hidden == 0)
        #expect(Fence(lang: "swift", "let x = 1") == .source)
    }

    @Test internal func shellOutputFoldsToItsTailAndKeepsItsColours() {
        let output = (1...14).map { "line \($0)" }.joined(separator: "\n")
        guard case .output(let lines) = Fence(lang: "console", output) else { return #expect(Bool(false)) }
        let tail = Folded.tail(lines)
        #expect(tail.lines == (7...14).map { "line \($0)" } && tail.hidden == 6)
        #expect(Fence(lang: "", "\u{1B}[32mok\u{1B}[0m") == .output(["\u{1B}[32mok\u{1B}[0m"]))
        #expect(
            ANSI.runs("\u{1B}[1mBuild\u{1B}[0m \u{1B}[2m◇\u{1B}[0m \u{1B}[1;91mfailed\u{1B}[39m!")
                == [
                    ANSI.Run("Build", bold: true), ANSI.Run(" "), ANSI.Run("◇", colour: 90), ANSI.Run(" "),
                    ANSI.Run("failed", colour: 31, bold: true), ANSI.Run("!", bold: true),
                ])
    }

    @Test internal func aMessageCopiesAsPlainText() {
        let reply = Markdown.blocks(
            "Questions for you:\n1. **HTML** or `SwiftUI`?\n   - [x] a\n\n## Then\n> q\n\n| A | B |\n|--|--|\n| 1 | 2 |\n\n---\n\n```sh\nls\n```"
        )
        #expect(
            Item.agent(id: 1, markdown: reply).plainText
                == "Questions for you:\n\n1. HTML or SwiftUI?\n  [x] a\n\nThen\n\nq\n\nA | B\n1 | 2\n\n---\n\nls")
        #expect(Item.you(id: 2, text: "hi **there**").plainText == "hi **there**")
        #expect(Item.note(id: 3, text: "Model set").plainText == nil)
        #expect(
            Item.said(id: 4, speaker: "Ada", time: "9:41", markdown: Markdown.blocks("**ok** ![i](https://a.b/i.png)"))
                .plainText == "ok i")
        #expect(Item.divider(id: 5, text: "Round 1").plainText == nil)
    }
}
