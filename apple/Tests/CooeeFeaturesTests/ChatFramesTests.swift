import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct ChatFramesTests {
    private let calls = Calls()

    @Test internal func aFrameLoadsOnceAndOneGoneAfterItsDaysIsNotAskedForAgain() async {
        let calls = calls
        let state = ChatFeature.State(server: mini, target: .thread("a1"), persona: Persona(coder))
        let store = TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.threads.frame = { _, thread, name in
                calls.record("frame \(thread) \(name)")
                guard name == "f1" else { throw URLError(.fileDoesNotExist) }
                return Data("pixels".utf8)
            }
        }
        await store.send(.frame("f1"))
        await store.receive(\.framed) { $0.frames["f1"] = Data("pixels".utf8) }
        await store.send(.frame("f1"))
        await store.send(.frame("gone"))
        await store.receive(\.framed) { $0.frames["gone"] = Data() }
        await store.send(.frame("gone"))
        #expect(calls.all == ["frame a1 f1", "frame a1 gone"])
    }

    @Test internal func aNewChatHasNoFramesToLoad() async {
        let store = TestStore(
            initialState: ChatFeature.State(server: mini, target: .new(coder), persona: Persona(coder))
        ) {
            ChatFeature()
        }
        await store.send(.frame("f1"))
    }
}
