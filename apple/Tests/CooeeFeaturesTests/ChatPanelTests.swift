import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct ChatPanelTests {
    nonisolated private static let queue = Place(
        plugin: "drover", key: "queue", label: "Queue", screen: "/screens/queue")
    nonisolated private static let drover = PluginRow(
        name: "drover", version: "1", commit: "d", places: [PluginPlace(kind: "header-strip", label: "Queue")],
        permissions: .init()
    )

    private let calls = Calls()

    private func app() -> TestStoreOf<AppFeature> {
        var state = AppFeature.State(layout: testLayout, server: mini)
        state.home?.rows = [row("t1")]
        state.home?.personas = [coder]
        state.path = ["t1"]
        state.chat = ChatFeature.State(server: mini, target: .thread("t1"), persona: Persona(coder))
        let calls = calls
        return TestStore(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.threads.stop = { _, thread in calls.record("stop \(thread)") }
        }
    }

    @Test internal func aPickInTheSheetRunsOnceTheSheetHasGone() async {
        let store = app()
        await store.send(.chat(.panel(.show(true)))) { $0.chat?.panel = true }
        await store.send(.chat(.panel(.pick(.stop)))) {
            $0.chat?.panel = false
            $0.chat?.picked = .stop
        }
        await store.send(.chat(.panel(.dismissed))) { $0.chat?.picked = nil }
        await store.receive(\.chat.compose.stop)
        await store.send(.chat(.panel(.dismissed)))
        #expect(calls.all == ["stop t1"])
    }

    @Test internal func allIssuesOpensTheWorkflowSheetOnceThePanelHasGone() async {
        let store = app()
        await store.send(.chat(.panel(.show(true)))) { $0.chat?.panel = true }
        await store.send(.chat(.panel(.pick(.workflow)))) {
            $0.chat?.panel = false
            $0.chat?.picked = .workflow
        }
        await store.send(.chat(.panel(.dismissed))) { $0.chat?.picked = nil }
        await store.receive(\.chat.workflow.show) { $0.chat?.workflowOpen = true }
    }

    @Test internal func editOpensThePersonaSheetAndNewChatStartsOneWithThePersona() async {
        let store = app()
        await store.send(.chat(.panel(.show(true)))) { $0.chat?.panel = true }
        await store.send(.chat(.panel(.pick(.edit)))) {
            $0.chat?.panel = false
            $0.chat?.picked = .edit
        }
        await store.send(.chat(.panel(.dismissed))) { $0.chat?.picked = nil }
        await store.receive(\.chat.delegate.persona)
        await store.receive(\.edit) { $0.persona = PersonaFeature.State(server: mini, persona: coder) }
        await store.send(.persona(.dismiss)) { $0.persona = nil }
        await store.send(.chat(.panel(.run(.newChat))))
        await store.receive(\.chat.delegate.start)
        await store.receive(\.start) {
            $0.path = ["new:coder"]
            $0.chat = ChatFeature.State(server: mini, target: .new(coder), persona: Persona(coder))
        }
    }

    @Test internal func theColumnOpensAPlaceOrAttachesAPluginAtOnce() async {
        let store = app()
        await store.send(.chat(.panel(.run(.open(Self.queue)))))
        await store.receive(\.chat.plugins.open) {
            $0.chat?.screen = ScreenFeature.State(server: mini, place: Self.queue)
        }
        await store.send(.chat(.screen(.dismiss))) { $0.chat?.screen = nil }
        await store.send(.chat(.panel(.run(.attach(Self.drover)))))
        await store.receive(\.chat.plugins.attach) {
            $0.chat?.attaching = AttachFeature.State(server: mini, persona: coder, plugin: Self.drover)
        }
    }
}
