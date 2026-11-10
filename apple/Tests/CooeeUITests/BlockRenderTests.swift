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

    private static func fixture(_ name: String) throws -> JSONValue {
        try JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: fixtures.appendingPathComponent(name)))
    }

    private static func height(_ block: JSONValue) -> Int {
        let view = BlocksView(block).frame(width: 402).environment(\.look, Look(palette: .farmLight, still: true))
        return ImageRenderer(content: view).cgImage?.height ?? 0
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
}
