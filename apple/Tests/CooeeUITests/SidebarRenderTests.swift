import ComposableArchitecture
import CooeeCore
import CooeeFeatures
@testable import CooeeUI
import Foundation
import SwiftUI
import Testing

@MainActor
internal struct SidebarRenderTests {
    nonisolated private static let queue = JSONValue(
        json: #"{"type": "screen", "blocks": [{"type": "row", "title": "Three lanes wait on review"}]}"#)
    private static let node = #"""
        , "slots": {"top": [{"type": "screen", "source": "drover", "path": "/screens/queue", "label": "Queue",
                             "place": {"wide": "column", "compact": "menu"}}]}
        """#

    private static func drawn(sidebar: String) throws -> Pixels {
        let json = #"""
            {"format": 1, "root": {"type": "nav", "slots": {
              "side": [{"type": "sidebar", "place": {"wide": "column", "compact": "menu"}\#(sidebar)}],
              "main": [{"type": "screen", "source": "core", "path": "/screens/home", "id": "home"},
                       {"type": "chat", "place": "push"}]}}}
            """#
        let layout = try CooeeCore.Layout(json: Data(json.utf8), etag: "")
        var state = AppFeature.State(layout: layout, server: Server(url: URL(filePath: "/"), token: "t"))
        for id in state.layout.screens.ids { state.layout.screens[id: id]?.screen = LiveScreen(queue) }
        let store = Store(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.plugins.screen = { _, _, _ in Self.queue }
            $0.screenCache.save = { _, _ in }
            $0.continuousClock = TestClock()
            $0.date.now = Date(timeIntervalSince1970: 0)
        }
        let view = SlotView(store: store, node: layout.resolved(for: .wide), compact: false)
            .frame(width: 1440, height: 900).environment(\.look, Look(palette: .farmDark, still: true))
        let image = try #require(ImageRenderer(content: view).cgImage)
        return try #require(Pixels(image))
    }

    private static func changed(_ one: Pixels, _ other: Pixels, columns: Range<Int>) -> Bool {
        (0..<one.height).contains { row in
            columns.contains { column in
                let start = (row * one.width + column) * 4
                return one.bytes[start..<start + 4] != other.bytes[start..<start + 4]
            }
        }
    }

    @Test internal func aPluginScreenMovedToTheSidebarIsDrawnInItsColumnAndNowhereElse() throws {
        let moved = try Self.drawn(sidebar: Self.node)
        let plain = try Self.drawn(sidebar: "")
        #expect(Self.changed(moved, plain, columns: 0..<359))
        #expect(!Self.changed(moved, plain, columns: 361..<moved.width))
    }
}
