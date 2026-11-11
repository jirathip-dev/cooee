@testable import CooeeCore
import Testing

internal struct ItemsTests {
    private let items = [
        APIItem(id: 1, time: 1, kind: "you", text: "Fix it", files: ["1-0-log.txt"]),
        APIItem(id: 2, time: 2, kind: "tool", title: "Read a", status: "completed"),
        APIItem(id: 3, time: 3, kind: "tool", title: "Run b", status: "in_progress"),
        APIItem(id: 4, time: 4, kind: "tool", title: "Run c", status: "failed"),
        APIItem(id: 5, time: 5, kind: "approval", plugin: "edgar", block: .object(["type": .string("card")])),
        APIItem(id: 6, time: 6, kind: "approval"),
        APIItem(id: 7, time: 7, kind: "note", text: "Model set"),
        APIItem(id: 8, time: 8, kind: "ended", text: ""),
        APIItem(id: 9, time: 9, kind: "plan"),
        APIItem(id: 10, time: 10, kind: "agent", text: "Done."),
        APIItem(id: 11, time: 11, kind: "you"),
        APIItem(id: 12, time: 12, kind: "agent"),
    ]

    @Test internal func toolsFoldCardsAreBlocksAndNotesShow() {
        let drawn = ChatItems.build(items, live: "")
        #expect(
            drawn == [
                .you(id: 1, text: "Fix it", files: ["1-0-log.txt"]),
                .tools(
                    id: 2, rows: [ToolRow("Read a", "done"), ToolRow("Run b", "running"), ToolRow("Run c", "failed")]),
                .card(id: 5, poster: "edgar", block: .object(["type": .string("card")])),
                .note(id: 7, text: "Model set"),
                .agent(id: 10, markdown: [.text("Done.")]),
                .you(id: 11, text: "", files: []),
                .agent(id: 12, markdown: []),
            ]
        )
        #expect(drawn.map(\.id) == [1, 2, 5, 7, 10, 11, 12])
        #expect(ToolRow("Run b", "running").running && !ToolRow("Read a", "done").running)
    }

    private static let card = JSONValue.object(["type": .string("card")])
    private static let steps = (2...6).map { ToolRow("Step \($0)", "done") }
    private static let turn: [APIItem] = {
        let tools = (2...6).map {
            APIItem(id: $0, time: Int64($0) * 1000, kind: "tool", title: "Step \($0)", status: "completed")
        }
        return [APIItem(id: 1, time: 1000, kind: "you", text: "Fix it")] + tools[0..<3]
            + [APIItem(id: 7, time: 7000, kind: "approval", block: card)] + tools[3...]
            + [
                APIItem(id: 8, time: 8000, kind: "ended", text: "The agent exited (1)."),
                APIItem(id: 9, time: 92000, kind: "agent", text: "Fixed."),
            ]
    }()

    @Test internal func aChatStyleTurnIsItsReplyAndOneFoldedRowWithApprovalsAndErrorsInline() {
        #expect(
            ChatItems.build(Self.turn, live: "", style: .chat) == [
                .you(id: 1, text: "Fix it", files: []),
                .card(id: 7, poster: nil, block: Self.card),
                .note(id: 8, text: "The agent exited (1)."),
                .agent(id: 9, markdown: [.text("Fixed.")]),
                .trace(id: 2, steps: Self.steps, seconds: 90),
            ]
        )
        #expect(
            ChatItems.build(Self.turn, live: "") == [
                .you(id: 1, text: "Fix it", files: []),
                .tools(id: 2, rows: Array(Self.steps[0..<3])),
                .card(id: 7, poster: nil, block: Self.card),
                .tools(id: 5, rows: Array(Self.steps[3...])),
                .note(id: 8, text: "The agent exited (1)."),
                .agent(id: 9, markdown: [.text("Fixed.")]),
            ]
        )
    }

    @Test internal func aWorkingChatStyleTurnHidesItsStepsAndNamesTheCurrentOne() {
        let working = Array(Self.turn.dropLast(2))
        #expect(
            ChatItems.build(working, live: "", style: .chat, working: true) == [
                .you(id: 1, text: "Fix it", files: []), .card(id: 7, poster: nil, block: Self.card),
            ])
        let again = APIItem(id: 10, time: 9000, kind: "you", text: "Again")
        #expect(
            ChatItems.build(working + [again], live: "", style: .chat).suffix(2) == [
                .trace(id: 2, steps: Self.steps, seconds: 4), .you(id: 10, text: "Again", files: []),
            ])
        #expect(ChatItems.step(working) == "Step 6")
        #expect(ChatItems.step(Self.turn) == nil)
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

    @Test internal func theSameNoteTwiceInARowIsOneRow() {
        let stopped = [
            APIItem(id: 1, time: 1, kind: "ended", text: "The agent stopped."),
            APIItem(id: 2, time: 2, kind: "ended", text: "The agent stopped."),
            APIItem(id: 3, time: 3, kind: "ended", text: "The agent stopped."),
            APIItem(id: 4, time: 4, kind: "you", text: "Again"),
            APIItem(id: 5, time: 5, kind: "ended", text: "The agent stopped."),
        ]
        #expect(
            ChatItems.build(stopped, live: "") == [
                .note(id: 1, text: "The agent stopped."), .you(id: 4, text: "Again"),
                .note(id: 5, text: "The agent stopped."),
            ]
        )
    }
}
