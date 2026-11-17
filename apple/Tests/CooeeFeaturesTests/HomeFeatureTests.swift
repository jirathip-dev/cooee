import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct HomeFeatureTests {
    private enum Load: Sendable {
        case rows([ThreadRow])
        case failure(APIError)
    }

    private let clock = TestClock()

    private func store(_ loads: [Load]) -> TestStoreOf<HomeFeature> {
        let queue = Queue(loads)
        return TestStore(initialState: HomeFeature.State(server: mini)) {
            HomeFeature()
        } withDependencies: { [clock] in
            $0.continuousClock = clock
            $0.threads.home = { _ in
                switch queue.next() {
                case .rows(let rows): return HomeAnswer(screen: homeScreen, threads: rows, personas: [coder])
                case .failure(let error): throw error
                case nil: return HomeAnswer(screen: homeScreen, threads: [], personas: [coder])
                }
            }
            $0.plugins.installed = { _ in [drover] }
        }
    }

    @Test internal func homeReadsItsChatsEveryFewSecondsUntilTheServerForgetsThisDevice() async {
        let rows = [row("t1", state: "working"), row("old", archived: true)]
        let store = store([
            .rows(rows), .failure(APIError(status: 500, text: "Locked.")), .failure(APIError(status: 401, text: "")),
        ])
        await store.send(.task)
        await store.receive(\.loaded) {
            $0.screen = homeScreen
            $0.rows = [rows[0]]
            $0.personas = [coder]
            $0.loading = false
            $0.plugins = [drover]
        }
        await clock.advance(by: .seconds(5))
        await store.receive(\.failed) { $0.error = "Locked." }
        await clock.advance(by: .seconds(5))
        await store.receive(\.delegate.unauthorized)
        await store.finish()
    }

    @Test internal func homeListsPersonasChatsOnlyAndALaneFallsIntoOthersUntilAPluginClaimsIt() async {
        let rows = [row("t1"), lane("l1"), row("loose", persona: "")]
        let store = store([])
        store.dependencies.threads.home = { _ in
            HomeAnswer(screen: homeScreen, threads: rows, personas: [coder, conductor])
        }
        store.dependencies.plugins.installed = { _ in [] }
        await store.send(.refresh)
        await store.receive(\.loaded) {
            $0.loading = false
            $0.screen = homeScreen
            $0.rows = rows
            $0.personas = [coder, conductor]
        }
        var lists = store.state.lists(now: epoch)
        #expect(lists.chats.map(\.id) == ["t1"] && lists.workflow.isEmpty)
        #expect(lists.others.map(\.id) == ["l1", "loose"])
        await store.send(.loaded(homeScreen, rows, [coder, conductor], [drover])) { $0.plugins = [drover] }
        lists = store.state.lists(now: epoch)
        #expect(lists.chats.map(\.id) == ["t1"] && lists.others.map(\.id) == ["loose"])
        #expect(lists.workflow.mapValues { $0.map(\.id) } == ["cond": ["l1"]])
    }

    @Test internal func aRefreshLoadsOnceAndClearsTheError() async {
        let store = store([.rows([row("t1")])])
        await store.send(.failed("Locked.")) {
            $0.error = "Locked."
            $0.loading = false
        }
        await store.send(.refresh)
        await store.receive(\.loaded) {
            $0.screen = homeScreen
            $0.rows = [row("t1")]
            $0.personas = [coder]
            $0.plugins = [drover]
            $0.error = nil
        }
        #expect(store.state.pinned.map(\.state) == [.done])
        #expect(store.state.everyone.map(\.id) == ["coder"])
        #expect(store.state.lists(now: epoch).chats.map(\.id) == ["t1"])
        #expect(store.state.persona("coder") == coder && store.state.persona("x") == nil)
        await store.send(.delegate(.unauthorized))
    }

    @Test internal func offlineHomeHoldsNoScreenSoTheBundledOneDraws() async {
        let store = store([.failure(APIError(status: 502, text: "Offline."))])
        await store.send(.refresh)
        await store.receive(\.failed) {
            $0.error = "Offline."
            $0.loading = false
        }
        #expect(store.state.screen == nil && store.state.rows.isEmpty)
    }
}
