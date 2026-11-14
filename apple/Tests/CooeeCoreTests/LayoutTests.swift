import CooeeCore
import Foundation
import Testing

internal struct LayoutTests {
    private static func fixture(_ name: String) throws -> Data {
        let url = try #require(
            Bundle.module.url(forResource: name, withExtension: "json", subdirectory: "Fixtures/layouts"))
        return try Data(contentsOf: url)
    }

    private static func refusal(_ json: Data) -> LayoutError? {
        do {
            _ = try Layout(json: json, etag: "e")
            return nil
        } catch {
            return error
        }
    }

    private static let deep = "root" + String(repeating: ".slots.main[0]", count: 6)

    @Test(arguments: [
        ("newer-format", LayoutError.format(2)),
        ("unknown-type", .unknownType(path: "root.slots.main[0]", type: "carousel")),
        ("root-not-nav", .unknownType(path: "root", type: "screen")),
        ("bad-screen", .badScreen(path: "root.slots.main[0]")),
        ("unknown-place", .badPlace(path: "root.slots.side[0]", place: "colum")),
        ("unknown-slot", .unknownSlot(path: "root.slots.main[0]", slot: "header")),
        ("unknown-config", .unknownConfig(path: "root.slots.main[0]", key: "colour")),
        ("duplicate-id", .duplicateId("x")),
        ("cycle", .tooDeep(path: deep)),
    ])
    internal func aBadLayoutIsRefusedWithWhereAndWhy(_ name: String, _ expected: LayoutError) throws {
        #expect(Self.refusal(try Self.fixture(name)) == expected)
    }

    @Test internal func aFileThatIsNotJSONIsRefused() throws {
        guard case .json = Self.refusal(try Self.fixture("not-json")) else {
            Issue.record("not refused as JSON")
            return
        }
        #expect(Self.refusal(Data(count: 64 * 1024 + 1)) == .json("over 65536 bytes"))
    }

    @Test internal func moreThanTwoHundredNodesIsRefused() {
        let rows = (0..<200).map { #"{"type": "screen", "source": "core", "path": "/screens/r\#($0)"}"# }
        let json = #"{"format": 1, "root": {"type": "nav", "slots": {"main": [\#(rows.joined(separator: ","))]}}}"#
        #expect(Self.refusal(Data(json.utf8)) == .tooMany(200))
    }

    @Test internal func aLayoutCannotDropReSourceOrRePlaceTheLockedScreens() throws {
        let replaced = try Layout(json: try Self.fixture("locked-replaced"), etag: "e")
        let minimal = try Layout(json: Data(#"{"format": 1, "root": {"type": "nav"}}"#.utf8), etag: "e")
        #expect(replaced.root.slots["main"]?.map(\.id) == ["home", "pairing"])
        #expect(replaced.root.slots["menu"]?.map(\.id) == ["menu", "reset-layout"])
        #expect(replaced.root.slots["main"]?.last == minimal.root.slots["main"]?.last)
        #expect(replaced.root.slots["menu"]?.last == minimal.root.slots["menu"]?.last)
        let pairing = try #require(minimal.root.slots["main"]?.last)
        #expect(pairing.locked && pairing.content == .screen(.core, path: "/screens/pairing"))
        #expect(pairing.resolvedPlaces == [.push, .push])
        let reset = try #require(minimal.root.slots["menu"]?.last)
        #expect(reset.locked && reset.resolvedPlaces == [.menu, .menu])
    }

    @Test internal func aMissingSizeInheritsTheOtherAndResolvesPerSize() throws {
        let json = """
            {"format": 1, "theme": "farm", "root": {"type": "nav", "slots": {"side": [
              {"type": "sidebar", "label": "Chats", "place": {"wide": "column"}, "config": {"width": 300}},
              {"type": "screen", "source": "drover", "path": "/screens/queue", "place": {"compact": "sheet"}}]}}}
            """
        let layout = try Layout(json: Data(json.utf8), etag: "abc")
        #expect(layout.format == 1 && layout.theme == "farm" && layout.etag == "abc")
        let side = try #require(layout.root.slots["side"])
        #expect(side.map(\.id) == ["sidebar", "drover:/screens/queue"])
        #expect(side[0].label == "Chats" && side[0].config == ["width": .number(300)])
        #expect(side.map(\.resolvedPlaces) == [[.column, .column], [.sheet, .sheet]])
        #expect(side[1].content == .screen(.plugin("drover"), path: "/screens/queue"))
        #expect(layout.resolved(for: .wide).place == .inline)
    }
}

extension LayoutNode<Placement> {
    fileprivate var resolvedPlaces: [LayoutPlace] { [place.on(.compact), place.on(.wide)] }
}
