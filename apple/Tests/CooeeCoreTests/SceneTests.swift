import CooeeCore
import Foundation
import Testing

internal struct SceneTests {
    private static let fixtures = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("docs/design/queue-scene/fixtures")

    private static func fixture(_ name: String) throws -> JSONValue {
        try JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: fixtures.appendingPathComponent(name)))
    }

    private static func find(_ id: String, in value: JSONValue) -> JSONValue? {
        if value.id == id { return value }
        return (value.items + value.fields.values).lazy.compactMap { find(id, in: $0) }.first
    }

    @Test internal func lanesStandInTheirPlacesGateFirstAndKeepTheirOrderWithinOne() throws {
        let lanes = try #require(Self.find("lanes", in: try Self.fixture("queue.json"))).list("lanes")
        let placed = SceneLane.placed(lanes.reversed())
        #expect(placed.map(\.id) == ["l-730", "l-655", "l-609", "l-722", "l-641", "l-708", "l-702"])
        #expect(placed.map(\.state.word).first == "at the gate" && placed[1].state.word == "refused")
        #expect(placed[0].name == "#730 design" && placed[0].sprite.hasSuffix("-needs"))
        #expect(placed[1].sprite.hasSuffix("-fail"))
        #expect(placed[2].source == lanes[4])
    }

    @Test internal func aHorseKeepsItsIssuesCoatInTheStripAndItsRow() {
        #expect(SceneLane.coat("#641") == SceneLane.coat("#641 hosts bootstrap: an ambiguous machine label"))
        #expect(SceneLane.coat("#641") == PersonaLook.coat("#641"))
        let lane = SceneLane(.object(["id": .string("l"), "title": .string("#641"), "state": .string("lost")]))
        #expect(lane.state == .idle && lane.name == "#641" && lane.sprite == "horse-\(PersonaLook.coat("#641"))-idle")
        #expect(SceneLane(.null).id == "" && SceneLane(.null).title == "")
    }

    @Test internal func aMergeOrANewRoundUpdatesTheStripAndTheRowInPlace() throws {
        var screen = LiveScreen(try Self.fixture("queue.json"))
        for event in try Self.fixture("queue-stream.json").items { screen.apply(event: event) }
        let root = screen.root
        #expect(screen.seq == 43 && screen.count == 6)
        let rounds = try #require(Self.find("q-722", in: root)).list("rounds")
        #expect(rounds.map(\.id) == ["r-722-impl", "r-722-rev1"])
        #expect(rounds.first?.string("state") == "done" && rounds.first?.string("summary") == "✓ 2 commits")
        #expect(Self.find("q-708", in: root) == nil)
        let placed = SceneLane.placed(try #require(Self.find("lanes", in: root)).list("lanes"))
        #expect(!placed.contains { $0.id == "l-708" })
        #expect(placed.first { $0.id == "l-722" }?.name == "#722 review 1")
        #expect(placed.map(\.id) == ["l-730", "l-655", "l-641", "l-722", "l-609", "l-456", "l-702"])
    }

    @Test internal func aLanesHeadIsItsHorseItsLiveRoundsLaneAndItsPR() throws {
        let head = LaneHead(thread: "herdsman-impl-641", screen: try Self.fixture("queue.json"))
        #expect(head.lane?.name == "#641 fix 1" && head.lane?.state == .working)
        #expect(head.line == "Claude Code · opus 5.5 · mini")
        #expect(head.round.string("since") == "2026-11-09T09:35:00Z")
        #expect(head.pull?.string("label") == "PR #704")
        let unknown = LaneHead(thread: "nobody", screen: try Self.fixture("queue.json"))
        #expect(unknown.lane == nil && unknown.line.isEmpty && unknown.pull == nil)
    }
}
