import CooeeCore
import Foundation
import Testing

internal struct ScreenTests {
    private static let fixtures = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("docs/design/screens-as-data/fixtures")

    private static func fixture(_ name: String) throws -> JSONValue {
        try JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: fixtures.appendingPathComponent(name)))
    }

    private static func find(_ id: String, in value: JSONValue) -> JSONValue? {
        if value.id == id { return value }
        let children = value.items + value.fields.values
        return children.lazy.compactMap { find(id, in: $0) }.first
    }

    @Test internal func everyFixtureDecodesAndEncodesBackTheSame() throws {
        let names = try FileManager.default.contentsOfDirectory(atPath: Self.fixtures.path).filter {
            $0.hasSuffix(".json")
        }
        #expect(names.count == 10)
        for name in names {
            let value = try Self.fixture(name)
            let again = try JSONDecoder().decode(JSONValue.self, from: JSONEncoder().encode(value))
            #expect(again == value, "\(name)")
            #expect(value != .null && (value.type != "" || !value.items.isEmpty || !value.fields.isEmpty), "\(name)")
        }
    }

    @Test internal func jsonTextParsesAndAStampReadsBackAsItsDate() {
        let now = Date(timeIntervalSince1970: 1_800_000_000)
        #expect(JSONValue(json: #"{"type": "run", "n": 2}"#) == .object(["type": .string("run"), "n": .number(2)]))
        #expect(JSONValue(json: "{not json") == .null)
        #expect(JSONValue.string(Relative.stamp(in: 690, from: now)).date == now.addingTimeInterval(690))
    }

    @Test internal func queueStreamOpsPatchTheQueueByID() throws {
        var screen = LiveScreen(try Self.fixture("queue.json"))
        for event in try Self.fixture("queue-stream.json").items { screen.apply(event: event) }
        let root = screen.root
        #expect(screen.seq == 43 && screen.count == 6)
        #expect(Self.find("q-722", in: root)?["badge"]?["text"] == .string("review 1"))
        #expect(Self.find("q-722", in: root)?["subtitle"] == .string("impl ✓ · review 1 ▸ herdsman-rev-722 · mini"))
        #expect(Self.find("q-722", in: root)?["tap"] != nil)
        #expect(Self.find("q-708", in: root) == nil)
        #expect(Self.find("done-today", in: root)?["value"] == .string("7"))
        #expect(Self.find("q-456", in: root)?["badge"]?["text"] == .string("impl"))
        #expect(Self.find("q-456", in: root)?["swipe"] == nil)
        #expect(Self.find("q-686", in: root)?["subtitle"]?["tone"] == .string("warn"))
        #expect(root.type == "screen" && root["title"] == .string("Queue"))
    }

    @Test internal func insertGoesBeforeAfterOrAtTheEndAndScreenAndDotReplaceTheWhole() throws {
        let row = { (id: String) in JSONValue.object(["type": .string("row"), "id": .string(id)]) }
        let list = JSONValue.object([
            "type": .string("list"), "id": .string("l"), "rows": .array([row("a"), row("b")]),
        ])
        var screen = LiveScreen(.object(["type": .string("screen"), "blocks": .array([list])]))
        let insert = { (id: String, place: [String: JSONValue]) in
            JSONValue.object(["op": .string("insert"), "into": .string("l"), "block": row(id)].merging(place) { $1 })
        }
        screen.apply(insert("c", [:]))
        screen.apply(insert("d", ["before": .string("a")]))
        screen.apply(insert("e", ["after": .string("a")]))
        screen.apply(.object(["op": .string("unknown"), "id": .string("a")]))
        screen.apply(.object(["op": .string("remove")]))
        #expect(Self.find("l", in: screen.root)?["rows"]?.items.compactMap(\.id) == ["d", "a", "e", "b", "c"])
        screen.apply(.object(["op": .string("dot"), "on": .bool(true)]))
        screen.apply(.object(["op": .string("screen"), "screen": row("z")]))
        #expect(screen.dot && screen.root == row("z"))
    }

    @Test internal func anActionAnswersWithItsChoiceOrOtherTextAndItsOneValue() {
        let act = { (values: [String: JSONValue]) in
            BlockAction(action: "answer", values: values).answer.map { "\($0.option) \($0.value ?? "-")" }
        }
        #expect(act(["choice": .string("approve"), "sha": .string("3b7e")]) == "approve 3b7e")
        #expect(act(["choice": .string("revise"), "other": .string("smaller")]) == "smaller -")
        #expect(act(["choice": .string("save"), "other": .string(""), "KEY": .string("v")]) == "save v")
        #expect(act(["KEY": .string("v")]) == nil)
    }

    @Test internal func timesAreRelativeToNow() throws {
        let now = try Date("2026-11-09T09:41:00Z", strategy: .iso8601)
        let since = JSONValue.object(["since": .string("2026-11-09T09:32:00Z")])
        let until = JSONValue.object(["until": .string("2026-11-09T12:52:00Z")])
        #expect(since.text(at: now) == "9m" && until.text(at: now) == "3h")
        #expect(JSONValue.number(14).text(at: now) == "14" && JSONValue.bool(true).text(at: now) == nil)
        #expect(
            JSONValue.object([:]).text(at: now) == nil && Relative.text(-5) == "now" && Relative.text(90_000) == "1d")
        #expect(Relative.filled("lapses in {rel}", until: now.addingTimeInterval(660), at: now) == "lapses in 11m")
        #expect(Relative.filled("{rel}", until: nil, at: now) == "{rel}")
        #expect(JSONValue.string("x").number == nil && JSONValue.number(1).string == nil && JSONValue.null.bool == nil)
    }

    @Test internal func aDraggedAccountTakesThePlaceOfTheRowItIsDroppedOn() throws {
        let role = Self.fixtures.deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("fleets-config/fixtures/role.json")
        let screen = try JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: role))
        var lists: [JSONValue] = []
        func walk(_ value: JSONValue) {
            if value["reorder"] != nil { lists.append(value) }
            (value.items + value.fields.values).forEach(walk)
        }
        walk(screen)
        let accounts = try #require(lists.first)
        #expect(accounts.rowOrder(moving: "A", onto: "B") == ["C", "B", "A"])
        #expect(accounts.rowOrder(moving: "B", onto: "A") == ["B", "A", "C"])
        #expect(accounts.rowOrder(moving: "Z", onto: "A") == ["A", "C", "B"])
    }
}
