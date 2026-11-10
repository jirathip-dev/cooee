@testable import CooeeCore
import Testing

internal struct ItemsTests {
    private let items = [
        APIItem(id: 1, time: 1, kind: "you", text: "Fix it", files: ["1-0-log.txt"]),
        APIItem(id: 2, time: 2, kind: "tool", title: "Read a", status: "completed"),
        APIItem(id: 3, time: 3, kind: "tool", title: "Run b", status: "in_progress"),
        APIItem(id: 4, time: 4, kind: "tool", title: "Run c", status: "failed"),
        APIItem(id: 5, time: 5, kind: "approval", tool: "Edit src/lib.rs"),
        APIItem(id: 6, time: 6, kind: "approval"),
        APIItem(id: 7, time: 7, kind: "note", text: "Model set"),
        APIItem(id: 8, time: 8, kind: "ended", text: ""),
        APIItem(id: 9, time: 9, kind: "plan"),
        APIItem(id: 10, time: 10, kind: "agent", text: "Done."),
        APIItem(id: 11, time: 11, kind: "you"),
        APIItem(id: 12, time: 12, kind: "agent"),
    ]

    @Test internal func toolsFoldCardsWaitAndNotesShow() {
        let drawn = ChatItems.build(items, live: "")
        #expect(
            drawn == [
                .you(id: 1, text: "Fix it", files: ["1-0-log.txt"]),
                .tools(
                    id: 2, rows: [ToolRow("Read a", "done"), ToolRow("Run b", "running"), ToolRow("Run c", "failed")]),
                .note(id: 5, text: "A card waits: Edit src/lib.rs. Answer it on the web for now."),
                .note(id: 6, text: "A card waits: an ask. Answer it on the web for now."),
                .note(id: 7, text: "Model set"),
                .agent(id: 10, markdown: [.text("Done.")]),
                .you(id: 11, text: "", files: []),
                .agent(id: 12, markdown: []),
            ]
        )
        #expect(drawn.map(\.id) == [1, 2, 5, 6, 7, 10, 11, 12])
        #expect(ToolRow("Run b", "running").running && !ToolRow("Read a", "done").running)
    }

    @Test internal func theReplySoFarIsDrawnLastWithItsCaret() {
        let drawn = ChatItems.build([], live: "Done:\n\n```ts\nawait x();\n```\n- one\n* two\nafter\n\nlast")
        #expect(
            drawn == [
                .agent(
                    id: Int.max,
                    markdown: [
                        .text("Done:"), .code(lang: "ts", "await x();"), .bullets(["one", "two"]), .text("after"),
                        .text("last ▍"),
                    ]
                )
            ]
        )
    }

    @Test internal func markdownKeepsAnOpenFenceAndSplitsParagraphsFromBullets() {
        #expect(Markdown.blocks("intro\n- a\n```sh\nls") == [.text("intro"), .bullets(["a"]), .code(lang: "sh", "ls")])
        #expect(Markdown.blocks("") == [])
    }

    @Test internal func everyCardKeepsItsID() {
        let choice = Choice(title: "Allow?", detail: "fetch", options: ["Yes", "No"], recommended: 0)
        let run = RunCard(why: "why", command: "ls", sha: "abc", folder: "~", blast: "none", lapses: "11m")
        let cards: [Item] = [
            .choice(id: 1, choice), .config(id: 2, plugin: "p", title: "t", fields: [Field("k", "v")]),
            .secret(id: 3, plugin: "p", name: "KEY", why: "w"), .run(id: 4, run),
        ]
        #expect(cards.map(\.id) == [1, 2, 3, 4])
        #expect(choice.options.count == 2 && run.command == "ls" && Field("k", "v").value == "v")
    }
}
