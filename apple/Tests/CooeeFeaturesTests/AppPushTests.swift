import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct AppPushTests {
    private func store(_ calls: Calls) -> TestStoreOf<AppFeature> {
        var state = AppFeature.State(layout: testLayout, server: mini)
        state.home?.rows = [row("t1", state: "working")]
        state.home?.personas = [coder]
        return TestStore(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.push.showing = { calls.record("showing \($0 ?? "none")") }
        }
    }

    @Test internal func theOpenThreadIsTheOneWhosePushesShowNoBanner() async {
        let calls = Calls()
        let store = store(calls)
        let chat = ChatFeature.State(server: mini, target: .thread("t1"), persona: Persona(coder, state: .working))
        await store.send(.open("t1")) {
            $0.path = ["t1"]
            $0.chat = chat
            $0.home?.opened["coder"] = "t1"
        }
        await store.send(.start("coder")) {
            $0.path = ["new:coder"]
            $0.chat = ChatFeature.State(server: mini, target: .new(coder), persona: Persona(coder))
        }
        await store.send(.open("t1")) {
            $0.path = ["t1"]
            $0.chat = chat
        }
        await store.send(.back) {
            $0.path = []
            $0.chat = nil
        }
        await store.finish()
        #expect(calls.all == ["showing t1", "showing none", "showing t1", "showing none"])
    }
}
