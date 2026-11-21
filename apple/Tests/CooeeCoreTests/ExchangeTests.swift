@testable import CooeeCore
import Foundation
import Testing

internal struct ExchangeTests {
    private let personas = [("coder", "Coder"), ("res", "Researcher"), ("am", "Account Manager")].map {
        APIPersona(id: $0.0, name: $0.1, agent: "codex")
    }

    private func row(_ id: String, _ persona: String, askedBy: String = "", members: [String] = []) -> ThreadRow {
        ThreadRow(
            id: id, title: "T \(id)", agent: "codex", persona: persona, state: "done", last: "", updated: 0, unseen: 0,
            archived: false, members: members, askedBy: askedBy
        )
    }

    private func ask(_ outcome: String, note: String? = nil, secs: Int = 48) -> APIItem {
        var item = APIItem(id: 3, time: 0, kind: "ask", text: "Q3?")
        item.persona = "res"
        item.name = "Researcher"
        item.thread = "x1"
        item.outcome = outcome
        item.secs = secs
        item.note = note
        return item
    }

    @Test internal func anAskIsACardThatSaysItsOutcome() throws {
        let said = ChatItems.build([ask("replied", note: "Use the 10-Q")])
        let card = try #require(said.first)
        guard case .asked(3, let shown) = card else { throw CancellationError() }
        #expect(shown.line(asker: "Coder") == "✓ replied in 48s · with your note" && !shown.refused)
        #expect(card.plainText == nil && card.id == 3)
        let line = { (outcome: String) in Ask(self.ask(outcome)).map { ($0.line(asker: "Coder"), $0.refused) } }
        #expect(line("replied").map(\.0) == "✓ replied in 48s")
        #expect(Ask(ask("replied")).map { $0.line(asker: "Coder", depth: 2) } == "✓ replied in 48s · depth 2")
        #expect(line("asking")?.0 == "asking…" && line("timeout")?.0 == "timed out after 48s")
        #expect(line("not_allowed")?.0 == "Not sent: Coder may not ask Researcher" && line("not_allowed")?.1 == true)
        #expect(line("depth")?.0 == "Not sent: Researcher is past depth 2" && line("offline")?.1 == true)
        #expect(line("offline")?.0 == "Not sent: Researcher is offline" && line("bogus") == nil)
        var bare = ask("replied")
        bare.persona = nil
        #expect(ChatItems.build([bare]).isEmpty)
    }

    @Test internal func anExchangeKnowsItsAskerItsDepthAndTheAnswerersOwnChat() throws {
        let rows = [
            row("x2", "am", askedBy: "x1"), row("x1", "res", askedBy: "c1"), row("c1", "coder"), row("r1", "res"),
            row("room", "", members: ["res", "am"]),
        ]
        let first = try #require(Exchange.of("x1", rows: rows, personas: personas))
        #expect(first.asker.name == "Coder" && first.target.name == "Researcher" && first.from == "c1")
        #expect(first.title == "T c1" && first.depth == 1 && first.chat == "r1")
        #expect(first.meter == "depth 1 of 2 · Researcher could ask one more")
        let second = try #require(Exchange.of("x2", rows: rows, personas: []))
        #expect(second.depth == 2 && second.meter == "depth 2 of 2" && second.chat == nil && second.asker.name == "res")
        #expect(Exchange.of("c1", rows: rows, personas: personas) == nil)

        let listed = Listing.chats(rows: rows, personas: personas, now: Date(timeIntervalSince1970: 0))
        #expect(listed.map(\.id) == ["c1", "x1", "x2", "r1", "room"])
        #expect(listed.map(\.from) == ["", "c1", "x1", "", ""])
        #expect(listed.map { $0.asker?.name } == [nil, "Coder", "Researcher", nil, nil])
    }

    private func exchangeItems(at stamp: Int64) -> [APIItem] {
        [
            APIItem(id: 1, time: stamp, kind: "you", text: "Q3?", speaker: "coder"),
            APIItem(id: 2, time: stamp, kind: "tool", title: "search sec.gov", status: "completed"),
            APIItem(id: 3, time: stamp, kind: "tool", title: "fetch sec.gov", status: "completed"),
            APIItem(id: 4, time: stamp, kind: "you", text: "Use the 10-Q"),
            ask("replied"),
            APIItem(id: 6, time: stamp, kind: "agent", text: "$4.1M"),
            APIItem(id: 7, time: stamp, kind: "note", text: "returned to Coder with your note · 48s"),
        ]
    }

    @Test internal func aLinkedThreadAttributesTheQuestionTheToolsTheNoteAndTheReply() throws {
        let rows = [row("c1", "coder"), row("x1", "res", askedBy: "c1")]
        let exchange = try #require(Exchange.of("x1", rows: rows, personas: personas))
        let stamp: Int64 = 1_000_000
        let items = exchangeItems(at: stamp)
        let now = Date(timeIntervalSince1970: 1_000)
        let time = Clock.time(stamp)
        let drawn = LinkedItems.build(items, exchange: exchange, outbox: [Outgoing(id: -1, text: "more")], now: now)
        #expect(
            drawn == [
                .divider(id: Int.min, text: "Today \(time) · ask_persona"),
                .said(id: 1, speaker: "coder", time: "asked · \(time)", markdown: [.text("Q3?")]),
                .said(id: -2, speaker: "res", time: time, markdown: []),
                .tools(
                    id: 2, rows: [ToolRow(.other, "search sec.gov", "done"), ToolRow(.other, "fetch sec.gov", "done")]),
                .you(id: 4, text: "Use the 10-Q"),
                .asked(id: 3, try #require(Ask(ask("replied")))),
                .said(id: 6, speaker: "res", time: "replied · \(time)", markdown: [.text("$4.1M")]),
                .divider(id: 7, text: "returned to Coder with your note · 48s"),
                .outgoing(Outgoing(id: -1, text: "more")),
            ]
        )
        #expect(LinkedItems.build([], exchange: exchange, outbox: [], now: now).isEmpty)
    }

    @Test internal func anotherDayIsNamedByItsDate() throws {
        let utc = try #require(TimeZone(identifier: "UTC"))
        #expect(Clock.day(0, now: Date(timeIntervalSince1970: 86_400 * 3), in: utc) == "1 Jan 00:00")
        #expect(Clock.day(0, now: Date(timeIntervalSince1970: 60), in: utc) == "Today 00:00")
    }

    @Test internal func aRowAndAnAskDecodeFromTheServersJSON() throws {
        let row = try JSONDecoder().decode(
            ThreadRow.self,
            from: Data(
                #"{"id":"x1","title":"","agent":"","persona":"res","state":"","last":"","at":1,"unseen":0,"archived":false,"asked_by":"c1"}"#
                    .utf8)
        )
        #expect(row.askedBy == "c1")
        let item = try JSONDecoder().decode(
            APIItem.self,
            from: Data(
                #"{"id":3,"at":1,"kind":"ask","persona":"res","name":"Researcher","text":"Q3?","thread":"x1","outcome":"replied","secs":48,"note":"Use the 10-Q"}"#
                    .utf8)
        )
        #expect(
            Ask(item)
                == Ask(
                    persona: "res", name: "Researcher", text: "Q3?", thread: "x1", outcome: .replied, seconds: 48,
                    note: "Use the 10-Q"))
    }
}
