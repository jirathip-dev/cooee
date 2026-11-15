import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct RoomPinTests {
    private let calls = Calls()
    private let res = APIPersona(id: "res", name: "Researcher", agent: "codex")

    private func store(_ settings: Settings = Settings()) -> TestStoreOf<AppFeature> {
        var state = AppFeature.State(layout: testLayout, server: mini, settings: settings)
        var pinned = coder
        pinned.pinned = true
        state.home?.personas = [pinned, res]
        state.home?.rows = [
            ThreadRow(
                id: "kestrel", title: "Kestrel", agent: "", persona: "", state: "working", last: "Coder: Signed.",
                updated: 9, unseen: 1, archived: false, members: ["coder", "res"]
            ),
            ThreadRow(
                id: "c1", title: "Fix", agent: "codex", persona: "res", state: "done", last: "ok", updated: 3,
                unseen: 0, archived: false
            ),
        ]
        let calls = calls
        return TestStore(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.settingsStore.save = { calls.record("save \($0.pinnedRooms)") }
        }
    }

    private func pins(_ store: TestStoreOf<AppFeature>) -> [String] {
        guard let home = store.state.home else { return [] }
        return home.lists(now: epoch).pins(home.pinned, rooms: store.state.menu.settings.pinnedRooms).map(\.id)
    }

    @Test internal func aRoomIsAChatWithItsMembersAndLastSpeakerNeverInOthers() throws {
        let lists = try #require(store().state.home?.lists(now: epoch))
        #expect(lists.chats.map(\.id) == ["kestrel", "c1"] && lists.others.isEmpty)
        let room = try #require(lists.chats.first)
        #expect(room.members.map(\.name) == ["Coder", "Researcher"] && room.members.map(\.state) == [.idle, .done])
        #expect(room.row.title == "Kestrel" && room.row.last == "Coder: Signed.")
    }

    @Test internal func pinningARoomPutsItAfterThePinnedPersonasAndUnpinningTakesItOut() async {
        let store = store()
        #expect(pins(store) == ["coder"])
        await store.send(.menu(.pinRoom("kestrel"))) { $0.menu.settings.pin(room: "kestrel") }
        #expect(pins(store) == ["coder", "kestrel"])
        await store.send(.menu(.pinRoom("kestrel"))) { $0.menu.settings.pin(room: "kestrel") }
        #expect(pins(store) == ["coder"])
        #expect(calls.all == ["save [\"kestrel\"]", "save []"])
    }

    @Test internal func aPinnedRoomIsKeptAcrossLaunchesAndAChatIsNeverPinnedAsARoom() async throws {
        var settings = Settings()
        settings.pin(room: "kestrel")
        settings.pin(room: "c1")
        let kept = try JSONDecoder().decode(Settings.self, from: JSONEncoder().encode(settings))
        #expect(kept.pinnedRooms == ["kestrel", "c1"])
        #expect(pins(store(kept)) == ["coder", "kestrel"])
        let old = try JSONDecoder().decode(Settings.self, from: Data(#"{"scheme":"dark","style":"modern"}"#.utf8))
        #expect(old.pinnedRooms.isEmpty)
    }
}
