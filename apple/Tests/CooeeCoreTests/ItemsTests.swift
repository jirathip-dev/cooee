@testable import CooeeCore
import Foundation
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
        let drawn = ChatItems.build(items)
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
            ChatItems.build(Self.turn, style: .chat) == [
                .you(id: 1, text: "Fix it", files: []),
                .card(id: 7, poster: nil, block: Self.card),
                .note(id: 8, text: "The agent exited (1)."),
                .agent(id: 9, markdown: [.text("Fixed.")]),
                .trace(id: 2, steps: Self.steps, seconds: 90),
            ]
        )
        #expect(
            ChatItems.build(Self.turn) == [
                .you(id: 1, text: "Fix it", files: []),
                .tools(id: 2, rows: Array(Self.steps[0..<3])),
                .card(id: 7, poster: nil, block: Self.card),
                .tools(id: 5, rows: Array(Self.steps[3...])),
                .note(id: 8, text: "The agent exited (1)."),
                .agent(id: 9, markdown: [.text("Fixed.")]),
            ]
        )
    }

    @Test internal func aBrowserFrameIsADoneStepWithItsCaptionInTheThreadAndTheTrace() {
        var frame = APIItem(id: 2, time: 2000, kind: "frame", text: #"click "Sign in""#)
        frame.name = "f1"
        let you = APIItem(id: 1, time: 1000, kind: "you", text: "Go")
        let items = [you, frame, APIItem(id: 3, time: 3000, kind: "agent", text: "Done.")]
        let row = ToolRow(#"click "Sign in""#, "done", frame: "f1")
        let (asked, done) = (Item.you(id: 1, text: "Go", files: []), Item.agent(id: 3, markdown: [.text("Done.")]))
        #expect(ChatItems.build(items) == [asked, .tools(id: 2, rows: [row]), done])
        #expect(
            ChatItems.build(items, style: .chat) == [asked, done, .trace(id: 2, steps: [row], seconds: 1)])
    }

    @Test internal func aWorkingChatStyleTurnHidesItsStepsAndNamesTheCurrentOne() {
        let working = Array(Self.turn.dropLast(2))
        #expect(
            ChatItems.build(working, style: .chat, working: true) == [
                .you(id: 1, text: "Fix it", files: []), .card(id: 7, poster: nil, block: Self.card),
            ])
        let again = APIItem(id: 10, time: 9000, kind: "you", text: "Again")
        #expect(
            ChatItems.build(working + [again], style: .chat).suffix(2) == [
                .trace(id: 2, steps: Self.steps, seconds: 4), .you(id: 10, text: "Again", files: []),
            ])
        #expect(ChatItems.step(working) == "Step 6")
        #expect(ChatItems.step(Self.turn) == nil)
    }

    @Test internal func markdownKeepsAnOpenFenceAndSplitsParagraphsFromBullets() {
        #expect(Markdown.blocks("intro\n- a\n```sh\nls") == [.text("intro"), .bullets(["a"]), .code(lang: "sh", "ls")])
        #expect(Markdown.blocks("") == [])
    }

    @Test internal func aMessageCopiesAsPlainText() {
        let reply = Markdown.blocks("Questions for you:\n1. **HTML** or `SwiftUI`?\n\n- a\n\n```sh\nls\n```")
        #expect(Item.agent(id: 1, markdown: reply).plainText == "Questions for you:\n1. HTML or SwiftUI?\n\n• a\n\nls")
        #expect(Item.you(id: 2, text: "hi **there**").plainText == "hi **there**")
        #expect(Item.note(id: 3, text: "Model set").plainText == nil)
        #expect(Item.said(id: 4, speaker: "Ada", time: "9:41", markdown: [.text("**ok**")]).plainText == "ok")
        #expect(Item.divider(id: 5, text: "Round 1").plainText == nil)
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
            ChatItems.build(stopped) == [
                .note(id: 1, text: "The agent stopped."), .you(id: 4, text: "Again"),
                .note(id: 5, text: "The agent stopped."),
            ]
        )
    }
}

extension ItemsTests {
    @Test internal func aFrameTheAgentAskedForIsAScreenBlockAndAHandoffACard() throws {
        let json = """
            [{"id":2,"at":2000,"kind":"frame","text":"https://a.com/x","name":"f2","asked":true},
             {"id":3,"at":3000,"kind":"frame","text":"click","name":"f3"},
             {"id":4,"at":4000,"kind":"handoff","text":"Sign in to a.com","outcome":"waiting"},
             {"id":5,"at":5000,"kind":"handoff","text":"2FA","outcome":"handed_back","note":"done it"}]
            """
        let items = try JSONDecoder().decode([APIItem].self, from: Data(json.utf8))
        let shot = Item.screen(
            id: 2, .shot(frame: "f2", address: "https://a.com/x", time: Date(timeIntervalSince1970: 2)))
        let waiting = Item.screen(id: 4, .handoff(what: "Sign in to a.com", waiting: true, note: ""))
        let back = Item.screen(id: 5, .handoff(what: "2FA", waiting: false, note: "done it"))
        let row = ToolRow("click", "done", frame: "f3")
        #expect(ChatItems.build(items) == [shot, .tools(id: 3, rows: [row]), waiting, back])
        #expect(
            ChatItems.build(items, style: .chat) == [
                shot, waiting, back, .trace(id: 3, steps: [row], seconds: 0),
            ])
        #expect(shot.plainText == nil && shot.id == 2)
        #expect(ScreenItem(APIItem(id: 6, time: 0, kind: "frame")) == nil)
    }
}

internal struct CallItemsTests {
    private static func voice(_ id: Int, _ time: Int64, _ text: String) -> APIItem {
        var item = APIItem(id: id, time: time, kind: "you", text: text)
        item.via = "voice"
        return item
    }

    private let items = [
        APIItem(id: 1, time: 0, kind: "you", text: "Find Birch."),
        voice(2, 1_000, "What did the 10-Q say?"),
        APIItem(id: 3, time: 2_000, kind: "tool", title: "fetch sec.gov", status: "completed"),
        APIItem(id: 4, time: 3_000, kind: "agent", text: "Birch is 14%."),
        voice(5, 4_000, "Why?"),
        APIItem(id: 6, time: 5_000, kind: "tool", title: "read notes", status: "completed"),
        APIItem(id: 7, time: 253_000, kind: "agent", text: "Deferred."),
        APIItem(id: 8, time: 254_000, kind: "you", text: "Write it up."),
    ]

    private let call = Item.call(
        Call(
            id: 2, seconds: 252,
            lines: [
                Call.Line(you: true, text: "What did the 10-Q say?"), Call.Line(you: false, text: "Birch is 14%."),
                Call.Line(you: true, text: "Why?"), Call.Line(you: false, text: "Deferred."),
            ], tools: 2))

    @Test internal func voiceTurnsGroupIntoOneCallBlockUntilATypedMessage() {
        let rows = [ToolRow("fetch sec.gov", "done"), ToolRow("read notes", "done")]
        #expect(
            ChatItems.build(items)
                == [
                    .you(id: 1, text: "Find Birch."), call, .tools(id: 3, rows: rows),
                    .you(id: 8, text: "Write it up."),
                ])
        #expect(
            ChatItems.build(items, style: .chat)
                == [
                    .you(id: 1, text: "Find Birch."), call, .trace(id: 3, steps: rows, seconds: 3),
                    .you(id: 8, text: "Write it up."),
                ])
        #expect(call.plainText == "What did the 10-Q say?\nBirch is 14%.\nWhy?\nDeferred.")
    }
}

extension ItemsTests {
    @Test internal func aHandoffsLinkLinesAreChipsThatOpenTheOtherThreadAndALinkWithoutOneIsDropped() throws {
        let json = """
            [{"id":2,"at":2000,"kind":"link","text":"","thread":"t6","title":"Fix the build","out":true},
             {"id":3,"at":3000,"kind":"link","text":"filed #61","thread":"t5","title":"","out":false},
             {"id":4,"at":4000,"kind":"link","text":"orphan","thread":""}]
            """
        let items = try JSONDecoder().decode([APIItem].self, from: Data(json.utf8))
        let onward = ThreadLink(thread: "t6", title: "Fix the build", onward: true)
        let back = ThreadLink(text: "filed #61", thread: "t5", title: "", onward: false)
        #expect(ChatItems.build(items) == [.link(id: 2, onward), .link(id: 3, back)])
        #expect(onward.chip == "continued in Fix the build" && back.chip == "from t5")
        #expect(Item.link(id: 2, onward).plainText == nil && Item.link(id: 2, onward).id == 2)
    }
}
