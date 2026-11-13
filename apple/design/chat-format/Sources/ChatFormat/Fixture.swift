import Foundation

public enum ChatFixture {
    private static let design = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()

    public static let entries: [Entry] = [
        .you("why does the chat say you:HTML?"),
        .edit(Diff(file: "apple/Sources/CooeeCore/Item.swift", edit: true, lines: edit)),
        .run(run),
        .agent(reply),
    ]

    private static let edit = [
        "@@ -105,14 +105,17 @@ public struct Markdown {",
        " public struct Markdown {",
        "     private var out: [Block] = []",
        "     private var paragraph: [String] = []",
        "-    private var bullets: [String] = []",
        "+    private var list: [ListItem] = []",
        "+    private var quote: [String] = []",
        "     private var code: (lang: String, lines: [String])?",
        " ",
        "     private mutating func flush() {",
        "         if !paragraph.isEmpty {",
        "-            out.append(.text(paragraph.joined()))",
        "+            out.append(.text(paragraph.joined(separator: \"\\n\")))",
        "             paragraph = []",
        "         }",
        "+        if !quote.isEmpty { out.append(.quote(quote)) }",
        "     }",
        " }",
    ]

    private static let run = Run(
        command: "swift test --filter Items", exit: 1, seconds: 9.8,
        output: [
            "\u{1B}[1mBuilding for debugging...\u{1B}[0m",
            "[1/412] Compiling CooeeCore Item.swift",
            "[412/412] Linking CooeeCoreTests",
            "Build complete! (8.31s)",
            "\u{1B}[2m◇\u{1B}[0m Test run started.",
        ] + passed + [
            "\u{1B}[33m⚠︎\u{1B}[0m Test tableScrolls() \u{1B}[33mskipped\u{1B}[0m: needs a 390 frame",
            "\u{1B}[31m✘\u{1B}[0m Test snapshotFrames() recorded an issue at SnapshotTests.swift:62",
            "  \u{1B}[31mchat-streaming-dark: 3.1% of pixels differ\u{1B}[0m",
            "\u{1B}[31m✘\u{1B}[0m Test run with 24 tests in 3 suites \u{1B}[1;31mfailed\u{1B}[0m after 0.41s with 1 issue.",
        ]
    )

    private static let passed = [
        "paragraphKeepsHardBreaks", "blankLineSplits", "headingsBySize", "orderedListCounts", "nestedListIndents",
        "taskListChecks", "quoteEndsAtBlankLine", "inlineCodeAndLinks", "codeFenceKeepsLanguage", "diffFenceCounts",
        "ansiColours", "ruleAndImage",
    ].map { "\u{1B}[32m✔\u{1B}[0m Test \($0)() passed after 0.002s." }

    private static let reply: [Part] = [
        .heading(1, "Paragraphs keep their line breaks"),
        .paragraph(
            "Found it. The parser joined lines with nothing between them, so `you:` and `HTML` ran together.\n"
                + "Now a line break in a reply stays a line break, and *blank lines* still split **paragraphs**."
        ),
        .heading(2, "What changed"),
        .list(ListBlock(ordered: true, items: [
            ListItem("`Markdown.blocks` keeps `\\n` inside a paragraph"),
            ListItem("Lists and quotes end at a blank line", children: ListBlock(ordered: false, items: [
                ListItem("a nested list indents one step", children: ListBlock(ordered: false, items: [
                    ListItem("and one more for the third level")
                ])),
                ListItem("numbers count from the first item"),
            ])),
            ListItem("Tool output folds after 12 lines"),
        ])),
        .heading(3, "Checklist"),
        .list(ListBlock(ordered: false, items: [
            ListItem("Regression test for `you:HTML`", done: true),
            ListItem("Fix in `CooeeCore`", done: true),
            ListItem("Re-record the snapshot frames", done: false),
        ])),
        .quote("Lines are never joined: “you:HTML” is two lines.\n— #243"),
        .table(table),
        .paragraph("The list parser reads the marker first:"),
        .code(lang: "swift", code),
        .diff(Diff(file: "apple/Sources/CooeeUI/Items.swift", edit: false, lines: fence)),
        .rule,
        .image(alt: "Chat after the fix", path: design.appendingPathComponent("v1/chat-streaming-light.png").path,
            size: "804×1748"),
        .paragraph("One snapshot still differs, so I'm re-recording it next. Details in [PR #262](https://github.com/jirathip-dev/cooee/pull/262)."),
    ]

    private static let table = Table(
        header: ["Element", "Before", "After", "Test", "Frames"], trailing: [4],
        rows: [
            ["Paragraph", "joined", "line breaks kept", "`paragraphKeepsHardBreaks`", "6"],
            ["List", "`- ` only", "numbered, nested, tasks", "`nestedListIndents`", "6"],
            ["Code", "plain", "syntax colours", "`codeFenceKeepsLanguage`", "6"],
            ["Table", "—", "scrolls sideways", "`tableScrolls`", "3"],
        ]
    )

    private static let code = """
        mutating func item(_ line: String) -> ListItem? {
            let indent = line.prefix { $0 == " " }.count / 2
            guard let marker = Marker(line.dropFirst(indent * 2)) else { return nil }
            return ListItem(marker.text, depth: indent, done: marker.task) // "- [x] a"
        }
        """

    private static let fence = [
        "@@ -81,1 +81,1 @@",
        "-    Text(Self.marked(text)).ui(16, .regular, look)",
        "+    Text(Self.marked(text, breaks: true))",
    ]
}
