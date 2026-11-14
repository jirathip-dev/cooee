@testable import CooeeCore
import Foundation
import Testing

internal struct RoomTests {
    private let members = ["chief", "coder", "res"]
    private let now = Date(timeIntervalSince1970: 1_000)

    @Test internal func aRoundShowsWhoRepliedPassedAndReplies() {
        var round = RoomRound()
        round.asked(members, text: "What's still open?", when: now)
        #expect(members.map(round.state) == [.upNext, .upNext, .upNext] && round.running)
        round.began("chief")
        round.replied("chief")
        round.began("coder")
        round.began("res")
        #expect(members.map(round.state) == [.replied, .passed, .replying] && round.turn == 3)
        round.ended("cap")
        #expect(round.running)
        round.ended("done")
        #expect(members.map(round.state) == [.replied, .passed, .passed] && !round.running)
        #expect(round.passed == ["coder", "res"])
    }

    @Test internal func aMentionWaitsTheOthersAndStopKeepsTheReplies() {
        var round = RoomRound()
        round.asked(members, text: "@res check Q3", when: now)
        round.began("res")
        round.replied("res")
        round.began("chief")
        round.asked(members, text: "and pricing?", when: now)
        #expect(members.map(round.state) == [.replying, .upNext, .upNext] && round.turn == 0)
        round.ended("stop")
        #expect(members.map(round.state) == [.waiting, .waiting, .waiting] && round.passed.isEmpty)
        #expect(RoomRound().state("chief") == .waiting)
    }

    @Test internal func roomItemsAreAttributedWithADividerAfterEachRound() {
        let items = [
            APIItem(id: 1, time: 0, kind: "you", text: "Draft the agenda.", speaker: "owner"),
            APIItem(id: 2, time: 60_000, kind: "agent", text: "It's in kickoff.md.", speaker: "chief"),
            APIItem(id: 3, time: 120_000, kind: "you", text: "What's open?", speaker: "owner"),
            APIItem(id: 4, time: 180_000, kind: "tool", title: "fetch sec.gov", status: "completed", speaker: "res"),
        ]
        var round = RoomRound()
        round.asked(members, text: "What's open?", when: now)
        round.began("coder")
        round.ended("done")
        let built = RoomItems.build(items, names: ["coder": "Coder"], last: round)
        let said = Item.said(
            id: 2, speaker: "chief", time: Clock.time(60_000), markdown: [.text("It's in kickoff.md.")])
        #expect(built.count == 6 && built[1] == said)
        #expect(built[2] == .divider(id: -3, text: "round ended · 1 turn"))
        #expect(built[4] == .tools(id: 4, rows: [ToolRow("fetch sec.gov", "done")]))
        #expect(built[5] == .divider(id: Int.min, text: "round ended · 1 turn · Coder passed"))
        #expect(RoomItems.ended(4, passed: []) == "round ended · 4 turns")
    }

    @Test internal func theClockReadsHoursAndMinutesAndTheRoundsMinutesAndSeconds() throws {
        let utc = try #require(TimeZone(identifier: "UTC"))
        #expect(Clock.time(34_800_000, in: utc) == "09:40")
        #expect(Clock.round(134) == "2:14" && Clock.round(-3) == "0:00")
    }

    @Test internal func aRoomRowCarriesItsMembersInRoomOrder() {
        let row = ThreadRow(
            id: "r1", title: "Kestrel", agent: "", persona: "", state: "", last: "", updated: 0, unseen: 0,
            archived: false, members: ["res", "ghost"]
        )
        let res = APIPersona(id: "res", name: "Researcher", agent: "claude")
        let chats = Listing.chats(rows: [row], personas: [res], now: now)
        #expect(chats[0].members.map(\.name) == ["Researcher", "ghost"] && row.members == ["res", "ghost"])
    }

    @Test internal func aMemberMovesInRoomOrderAndItsPlaceReads() {
        #expect(RoomOrder.move(members, "res", to: 0) == ["res", "chief", "coder"])
        #expect(RoomOrder.move(members, "chief", to: 9) == ["coder", "res", "chief"])
        #expect(RoomOrder.move(members, "ghost", to: 0) == members)
        #expect([1, 2, 3, 4, 11, 22].map(RoomOrder.ordinal) == ["1st", "2nd", "3rd", "4th", "11th", "22nd"])
    }
}
