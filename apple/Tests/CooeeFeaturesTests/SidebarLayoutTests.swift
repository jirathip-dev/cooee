import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct SidebarLayoutTests {
    nonisolated private static let moved = #"""
        {"format": 1, "root": {"type": "nav", "slots": {"side": [{"type": "sidebar",
          "place": {"wide": "column", "compact": "menu"}, "slots": {"top": [{"type": "screen", "source": "drover",
          "path": "/screens/queue", "label": "Queue", "place": {"wide": "column", "compact": "menu"}}]}}]}}}
        """#
    nonisolated private static let bare = LayoutAnswer(
        etag: "e4", layout: .object(["format": .number(1), "root": .object(["type": .string("nav")])]))
    nonisolated private static let why = #"root.slots.main[1].slots.cards[0]: drover may not be placed "cards""#

    @Test internal func aScreenMovedToTheSidebarIsFollowedThereAndASkippedNodeIsReported() async throws {
        let answer = LayoutAnswer(
            etag: "e3", layout: try JSONDecoder().decode(JSONValue.self, from: Data(Self.moved.utf8)),
            skipped: [Self.why])
        let store = TestStore(initialState: LayoutFeature.State(server: mini, layout: testLayout)) {
            LayoutFeature()
        } withDependencies: {
            $0.layoutClient.fetch = { _, _ in answer }
            $0.layoutClient.events = { _ in .finished() }
            $0.layoutStore.save = { _ in }
        }
        await store.send(.task)
        await store.receive(\.fetched) {
            $0.layout = layout(Self.moved, etag: "e3")
            $0.screens = [
                ScreenFeature.State(server: mini, source: .plugin("drover"), path: "/screens/queue", label: "Queue")
            ]
            $0.broken = "layout.json \(Self.why) · not drawn"
        }
        await store.send(.fetched(Self.bare)) {
            $0.layout = layout(#"{"format":1,"root":{"type":"nav"}}"#, etag: "e4")
            $0.screens = []
            $0.broken = nil
        }
    }
}
