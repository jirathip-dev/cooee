import CooeeCore
import Foundation
import Testing

internal struct SidebarTests {
    private static let queue = #"""
        {"type": "screen", "source": "drover", "path": "/screens/queue", "id": "queue", "label": "Queue",
         "place": {"wide": "column", "compact": "menu"}}
        """#
    private static let lanes = #"{"type": "screen", "source": "drover", "path": "/screens/lanes", "id": "lanes"}"#

    private static func layout(sidebar place: String = #"{"wide": "column", "compact": "menu"}"#) throws -> Layout {
        let json = #"""
            {"format": 1, "root": {"type": "nav", "slots": {
              "side": [{"type": "sidebar", "place": \#(place), "slots": {"list": [\#(lanes)], "top": [\#(queue)]}}],
              "menu": [{"type": "screen", "source": "core", "path": "/screens/menu", "id": "menu", "place": "sheet"}]}}}
            """#
        return try Layout(json: Data(json.utf8), etag: "")
    }

    @Test internal func theSidebarIsAColumnOnWideAndItsNodesAreMenuRowsOnCompact() throws {
        let wide = try Self.layout().resolved(for: .wide)
        #expect(wide.sidebarRows(on: .wide).map(\.id) == ["queue", "lanes"])
        #expect(wide.menuRows(on: .wide).map(\.id) == ["menu"])
        let compact = try Self.layout().resolved(for: .compact).menuRows(on: .compact)
        #expect(compact.map(\.id) == ["menu", "queue", "lanes"])
        #expect(compact.first { $0.id == "queue" }?.place == .menu)
    }

    @Test internal func aSidebarNotPlacedForTheSizeLendsNoRows() throws {
        #expect(try Self.layout().resolved(for: .wide).sidebarRows(on: .compact).isEmpty)
        #expect(try Self.layout(sidebar: #""hidden""#).resolved(for: .wide).sidebarRows(on: .wide).isEmpty)
    }
}
