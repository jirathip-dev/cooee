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
            $0.threads.threads = { _ in
                switch queue.next() {
                case .rows(let rows): return rows
                case .failure(let error): throw error
                case nil: return []
                }
            }
            $0.threads.personas = { _ in [coder] }
        }
    }

    @Test internal func homeReadsItsChatsEveryFewSecondsUntilTheServerForgetsThisDevice() async {
        let rows = [row("t1", state: "working"), row("old", archived: true)]
        let store = store([
            .rows(rows), .failure(APIError(status: 500, text: "Locked.")), .failure(APIError(status: 401, text: "")),
        ])
        await store.send(.task)
        await store.receive(\.loaded) {
            $0.rows = [rows[0]]
            $0.personas = [coder]
        }
        await clock.advance(by: .seconds(5))
        await store.receive(\.failed) { $0.error = "Locked." }
        await clock.advance(by: .seconds(5))
        await store.receive(\.delegate.unauthorized)
        await store.finish()
    }

    @Test internal func aRefreshLoadsOnceAndClearsTheError() async {
        let store = store([.rows([row("t1")])])
        await store.send(.failed("Locked.")) { $0.error = "Locked." }
        await store.send(.refresh)
        await store.receive(\.loaded) {
            $0.rows = [row("t1")]
            $0.personas = [coder]
            $0.error = nil
        }
        #expect(store.state.pinned.map(\.state) == [.done])
        #expect(store.state.chats(now: epoch).map(\.id) == ["t1"])
        #expect(store.state.persona("coder") == coder && store.state.persona("x") == nil)
        await store.send(.delegate(.unauthorized))
    }
}
