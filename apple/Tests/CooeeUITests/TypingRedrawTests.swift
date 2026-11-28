import ComposableArchitecture
import CooeeCore
import CooeeFeatures
@testable import CooeeUI
import Foundation
import SwiftUI
import Testing

@MainActor
internal struct TypingRedrawTests {
    private static func busy() -> StoreOf<ChatFeature> {
        let server = Server(url: URL(filePath: "/mini"), token: "tok")
        var state = ChatFeature.State(
            server: server, target: .thread("t1"), persona: Persona(APIPersona(id: "cond", name: "Cond", agent: "x")))
        state.items = (1...200).map { id in
            guard id % 13 == 0 else { return APIItem(id: id, time: 0, kind: "agent", text: "Line \(id) of the thread") }
            let choice: JSONValue = .object(["type": .string("choice"), "id": .string("\(id)-answer")])
            return APIItem(id: id, time: 0, kind: "approval", plugin: "drover", block: choice)
        }
        state.turn = Turn(state: "working")
        state.live = LiveReply("streaming")
        state.loading = false
        return Store(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.desk.warm = { _, _ in }
        }
    }

    private static func redraws(_ store: StoreOf<ChatFeature>, after act: () -> Void) -> Bool {
        let screen = ChatScreen(store: store, back: true) {}
        let redrawn = LockIsolated(false)
        withObservationTracking {
            _ = screen.body
        } onChange: {
            redrawn.setValue(true)
        }
        act()
        return redrawn.value
    }

    @Test internal func typingInABusyChatRedrawsNoneOfItsThread() {
        let store = Self.busy()
        let typed = Self.redraws(store) {
            for letter in "a long draft typed fast" { store.send(.compose(.draftChanged(store.draft + "\(letter)"))) }
        }
        #expect(!typed)
        #expect(store.draft == "a long draft typed fast")
        #expect(Self.redraws(store) { store.send(.compose(.attach([Attachment(name: "a.png", bytes: Data())]))) })
    }
}
