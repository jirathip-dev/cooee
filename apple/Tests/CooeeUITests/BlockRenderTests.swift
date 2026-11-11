import CooeeCore
import CooeeUI
import Foundation
import SwiftUI
import Testing

@MainActor
internal struct BlockRenderTests {
    private static let fixtures = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("docs/design/screens-as-data/fixtures")

    private static func fixture(_ name: String, in folder: URL = fixtures) throws -> JSONValue {
        try JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: folder.appendingPathComponent(name)))
    }

    private static func fleets(_ name: String) throws -> JSONValue {
        try fixture(
            name,
            in: fixtures.deletingLastPathComponent().deletingLastPathComponent()
                .appendingPathComponent("fleets-config/fixtures"))
    }

    private static func height(_ block: JSONValue, _ style: LookStyle = .modern) -> Int {
        let look = Look(palette: .farmLight, style: style, still: true)
        let view = BlocksView(block).frame(width: 402).environment(\.look, look)
        return ImageRenderer(content: view).cgImage?.height ?? 0
    }

    private static func row(current state: String) -> JSONValue {
        let round = { (id: String, state: String) in
            JSONValue.object(["id": .string(id), "name": .string(id), "state": .string(state)])
        }
        let lane = JSONValue.object(["name": .string("lane"), "lines": .array([.string("$ cargo test")])])
        var current = round("fix 1", state).fields
        current["lane"] = lane
        return .object([
            "type": .string("row"), "id": .string("q-1"), "title": .string("#1"),
            "rounds": .array([round("impl", "done"), .object(current)]),
        ])
    }

    @Test(arguments: ["queue.json", "events.json", "fleets.json", "hello.json"])
    internal func everyScreenDraws(_ name: String) throws {
        #expect(Self.height(try Self.fixture(name)) > 200)
    }

    @Test(arguments: ["own-cards.json", "cards.json"])
    internal func everyCardDraws(_ name: String) throws {
        for card in try Self.fixture(name).items { #expect(Self.height(card) > 80) }
    }

    @Test internal func everyCatalogueBlockDrawsAndAnUnknownOneIsSkipped() throws {
        let drawn = try Self.fixture("blocks.json").items.compactMap { $0["block"] }.filter { $0.type != "web" }
        for block in drawn where block.type != "confirm" { #expect(Self.height(block) > 0, "\(block.type)") }
        #expect(Self.height(.object(["type": .string("a-newer-block")])) == 0)
    }

    @Test internal func theQueueDrawsItsStreamsOps() throws {
        let screen = try Self.fixture("queue.json")
        var queue = LiveScreen(screen)
        for event in try Self.fixture("queue-stream.json").items { queue.apply(event: event) }
        let patched = Self.height(queue.root)
        #expect(patched > 200 && patched < Self.height(screen))
    }

    @Test internal func theSceneStripDrawsOnlyInTheSceneLook() throws {
        let scene = try #require(Self.sceneQueue().list("blocks").first)
        #expect(scene.type == "scene")
        #expect(Self.height(scene, .scene) == 180)
        #expect(Self.height(scene, .modern) == 0 && Self.height(scene, .terminal) == 0)
    }

    @Test internal func aRowWhoseCurrentRoundNeedsYouOpensItsRoundsOthersStayFolded() {
        let folded = Self.height(Self.row(current: "working"))
        let open = Self.height(Self.row(current: "needs"))
        #expect(folded > 0 && open > folded * 2)
        #expect(
            Self.height(Self.row(current: "working"), .terminal) < Self.height(Self.row(current: "needs"), .terminal))
    }

    @Test(arguments: [LookStyle.modern, .terminal, .scene])
    internal func theQueueInSceneDrawsInEveryLook(_ style: LookStyle) throws {
        #expect(Self.height(try Self.sceneQueue(), style) > 1000)
    }

    private static func sceneQueue() throws -> JSONValue {
        let file = fixtures.deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("queue-scene/fixtures/queue.json")
        return try JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: file))
    }

    @Test(arguments: [
        "fleets.json", "fleet.json", "role.json", "global.json", "text.json", "review.json", "add.json", "stale.json",
    ])
    internal func everyFleetsConfigScreenDraws(_ name: String) throws {
        #expect(Self.height(try Self.fleets(name)) > 300)
    }

    @Test internal func everyProposalCardDraws() throws {
        let cards = try Self.fleets("chat.json").list("cards")
        #expect(cards.count == 2)
        for card in cards { #expect(Self.height(card) > 150) }
    }

    @Test internal func anEditorShowsEachErrorUnderItsLine() throws {
        let screen = try Self.fleets("text.json")
        let editor = try #require(screen.list("blocks").first { $0.type == "editor" })
        var clean = LiveScreen(editor)
        clean.apply(
            .object(["op": .string("merge"), "id": .string("toml"), "fields": .object(["errors": .array([])])]))
        #expect(Self.height(editor) > Self.height(clean.root))
    }

    @Test internal func aDiffDrawsEveryLineOfEveryHunk() throws {
        let hunk: JSONValue = .object(["at": .string("@@ 1 @@"), "lines": .array([.string("- a"), .string("+ b")])])
        let one: JSONValue = .object(["type": .string("diff"), "file": .string("fleet.toml"), "hunks": .array([hunk])])
        let both: JSONValue = .object([
            "type": .string("diff"), "file": .string("fleet.toml"), "hunks": .array([hunk, hunk]),
        ])
        #expect(Self.height(both) > Self.height(one) && Self.height(one) > 0)
    }
}
