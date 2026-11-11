import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct RoomFeatureTests {
    private let calls = Calls()
    private let cast = ["a", "b", "c", "d", "e", "f"].map { APIPersona(id: $0, name: $0.uppercased(), agent: "codex") }

    private func room(_ members: [String]) -> ChatFeature.State {
        let room = Room(title: "Kestrel", members: members, cast: cast)
        return ChatFeature.State(server: mini, target: .thread("r1"), persona: Persona(coder), room: room)
    }

    private func dependencies(_ values: inout DependencyValues) {
        let calls = calls
        values.threads.room = { _, title, members in
            calls.record("room \(title) \(members.joined(separator: ","))")
            if title == "bad" { throw APIError(status: 400, text: "a room has 2 to 5 members") }
            return "r1"
        }
        values.threads.seat = { _, room, persona, place, remove in
            calls.record("seat \(room) \(persona) \(place.map(String.init) ?? "end") \(remove)")
            if persona == "f" { throw APIError(status: 400, text: "a room has at most 5 members") }
            return remove ? ["b", "a"] : ["b", "a", persona]
        }
    }

    @Test internal func aNewRoomPicksUpToFiveInOrderAndIsMade() async {
        let state = NewRoomFeature.State(server: mini, personas: cast)
        let store = TestStore(initialState: state) {
            NewRoomFeature()
        } withDependencies: {
            dependencies(&$0)
        }
        await store.send(.create)
        await store.send(.title("Kestrel")) { $0.title = "Kestrel" }
        for id in ["a", "b", "c", "d", "e"] { await store.send(.toggle(id)) { $0.picked.append(id) } }
        await store.send(.toggle("f"))
        await store.send(.toggle("c")) { $0.picked = ["a", "b", "d", "e"] }
        await store.send(.move("e", 0)) { $0.picked = ["e", "a", "b", "d"] }
        await store.send(.create) { $0.creating = true }
        await store.receive(\.delegate.created)
        #expect(calls.all == ["room Kestrel e,a,b,d"])
        await store.send(.title("bad")) { $0.title = "bad" }
        await store.send(.create)
        await store.send(.delegate(.cancel))
    }

    @Test internal func aRefusedRoomSaysWhy() async {
        var state = NewRoomFeature.State(server: mini, personas: cast)
        state.title = "bad"
        state.picked = ["a", "b"]
        let store = TestStore(initialState: state) {
            NewRoomFeature()
        } withDependencies: {
            dependencies(&$0)
        }
        await store.send(.create) { $0.creating = true }
        await store.receive(\.failed) {
            $0.creating = false
            $0.error = "a room has 2 to 5 members"
        }
    }

    @Test internal func aRoomsStreamRunsItsRoundAndItsItemsAreAttributed() async {
        let store = TestStore(initialState: room(["a", "b"])) {
            ChatFeature()
        } withDependencies: {
            dependencies(&$0)
        }
        let asked = APIItem(id: 1, time: 1_000, kind: "you", text: "hi all", speaker: "owner")
        let reply = APIItem(id: 2, time: 2_000, kind: "agent", text: "Hello", speaker: "a")
        await store.send(.event(.item(asked))) {
            $0.items = [asked]
            $0.room?.round.asked(["a", "b"], text: "hi all", when: Date(timeIntervalSince1970: 1))
        }
        await store.send(.event(.roomTurn("a"))) { $0.room?.round.began("a") }
        await store.send(.event(.item(reply))) {
            $0.items = [asked, reply]
            $0.room?.round.replied("a")
        }
        await store.send(.event(.roomTurn("b"))) { $0.room?.round.began("b") }
        await store.send(.event(.roomStop("done"))) { $0.room?.round.ended("done") }
        #expect(store.state.room?.round.state("b") == .passed)
        #expect(store.state.drawn.last == .divider(id: Int.min, text: "round ended · 2 turns · B passed"))
        #expect(store.state.room?.names["a"] == "A")
    }

    @Test internal func aMemberIsInspectedMovedAddedAndTakenOut() async {
        let store = TestStore(initialState: room(["a", "b"])) {
            ChatFeature()
        } withDependencies: {
            dependencies(&$0)
        }
        await store.send(.inspect("a")) { $0.room?.inspecting = "a" }
        await store.send(.move("b", 0)) { $0.room?.members = ["b", "a"] }
        await store.receive(\.seated)
        await store.send(.seat("c"))
        await store.receive(\.seated) { $0.room?.members = ["b", "a", "c"] }
        await store.send(.unseat("c")) { $0.room?.inspecting = nil }
        await store.receive(\.seated) { $0.room?.members = ["b", "a"] }
        await store.send(.seat("f"))
        await store.receive(\.failed) { $0.error = "a room has at most 5 members" }
        #expect(calls.all == ["seat r1 b 0 false", "seat r1 c end false", "seat r1 c end true", "seat r1 f end false"])
    }

    @Test internal func aRoomIsMadeFromHomeAndOpensFromTheList() async {
        var state = AppFeature.State(server: mini)
        state.home?.personas = cast
        let (rows, cast) = ([roomRow()], cast)
        let store = TestStore(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.threads.threads = { _ in rows }
            $0.threads.personas = { _ in cast }
        }
        store.exhaustivity = .off
        await store.send(.room) { $0.newRoom = NewRoomFeature.State(server: mini, personas: self.cast) }
        await store.send(.newRoom(.delegate(.cancel))) { $0.newRoom = nil }
        await store.send(.room)
        await store.send(.newRoom(.title("Kestrel")))
        await store.send(.newRoom(.delegate(.created("r1", ["a", "b"])))) {
            $0.newRoom = nil
            $0.path = ["r1"]
        }
        #expect(store.state.chat?.room == Room(title: "Kestrel", members: ["a", "b"], cast: cast))
        await store.receive(\.home.loaded)
        await store.send(.back)
        await store.send(.open("r9"))
        #expect(store.state.chat?.room?.members == ["c", "a"] && store.state.chat?.room?.title == "T r9")
    }

    private func roomRow() -> ThreadRow {
        ThreadRow(
            id: "r9", title: "T r9", agent: "", persona: "", state: "", last: "", updated: 0, unseen: 0,
            archived: false, members: ["c", "a"]
        )
    }
}
