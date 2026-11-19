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
            $0.date.now = epoch
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

    @Test internal func homeLoadsOnEachChangeTheServerSaysUntilTheServerForgetsThisDevice() async {
        let rows = [row("t1", state: "working"), row("old", archived: true)]
        let store = store([
            .rows(rows), .failure(APIError(status: 500, text: "Locked.")), .failure(APIError(status: 401, text: "")),
            .failure(APIError(status: 401, text: "")),
        ])
        let (changes, changed) = AsyncStream.makeStream(of: Void.self)
        store.dependencies.threads.changes = { _ in changes }
        await store.send(.task)
        changed.yield()
        await store.receive(\.loaded) {
            ($0.screen, $0.rows, $0.personas, $0.plugins) = (homeScreen, [rows[0]], [coder], [drover])
            $0.loading = false
        }
        changed.yield()
        await store.receive(\.failed) {
            $0.error = "Locked."
            $0.sync = .reconnecting
        }
        await clock.advance(by: .seconds(3))
        await store.receive(\.toasted) { $0.error = nil }
        changed.yield()
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
        await store.send(
            .loaded(HomeAnswer(screen: homeScreen, threads: rows, personas: [coder, conductor]), [drover])
        ) { $0.plugins = [drover] }
        lists = store.state.lists(now: epoch)
        #expect(lists.chats.map(\.id) == ["t1"] && lists.others.map(\.id) == ["loose"])
        #expect(lists.workflow.mapValues { $0.map(\.id) } == ["cond": ["l1"]])
    }

    @Test internal func aRefreshLoadsOnceAndClearsTheError() async {
        let store = store([.rows([row("t1")])])
        await store.send(.failed("Locked.", nil)) {
            $0.error = "Locked."
            $0.loading = false
            $0.sync = .reconnecting
        }
        await store.send(.refresh)
        await store.receive(\.loaded) {
            $0.screen = homeScreen
            $0.rows = [row("t1")]
            $0.personas = [coder]
            $0.plugins = [drover]
            $0.error = nil
            $0.sync = .live
        }
        await clock.advance(by: .seconds(3))
        await store.receive(\.toasted)
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
            $0.sync = .reconnecting
        }
        #expect(store.state.screen == nil && store.state.rows.isEmpty)
        await clock.advance(by: .seconds(3))
        await store.receive(\.toasted) { $0.error = nil }
    }

    @Test internal func aPersonasRowCountsAndSumsAsTheServerDoesAndNamesTheTopicATapOpens() async {
        let unread = ThreadRow(
            id: "c2", title: "T c2", agent: "codex", persona: "cond", state: "done", last: "", updated: 1, unseen: 1,
            archived: false)
        let rows = [row("c1", persona: "cond"), unread, row("t1")]
        let counts = ["cond": TopicCount(topics: 5, unread: 3)]
        let store = store([])
        store.dependencies.threads.home = { _ in
            HomeAnswer(screen: homeScreen, threads: rows, personas: [coder, conductor], counts: counts)
        }
        await store.send(.refresh)
        await store.receive(\.loaded) {
            $0.loading = false
            $0.screen = homeScreen
            $0.rows = rows
            $0.personas = [coder, conductor]
            $0.plugins = [drover]
            $0.counts = counts
        }
        let shown = store.state.shown(store.state.lists(now: epoch))
        #expect(shown.map(\.id) == ["c2", "t1"])
        #expect(shown[0].topics == 5 && shown[0].row.unseen == 3 && shown[0].row.title == "T c2")
        #expect(shown[1].topics == 0 && store.state.opens("cond")?.id == "c2")
    }
}

extension HomeFeatureTests {
    @Test internal func offlineHomeDrawsTheListItLastLoadedWithItsTimeAndToastsOnlyTheFirstFailure() async {
        let kept = LockIsolated<KeptHome?>(nil)
        let online = store([.rows([row("t1")])])
        online.dependencies.kept.keepHome = { _, home in kept.setValue(home) }
        await online.send(.refresh)
        await online.receive(\.loaded) {
            ($0.screen, $0.rows, $0.personas, $0.plugins) = (homeScreen, [row("t1")], [coder], [drover])
            $0.loading = false
        }
        let offline = store([
            .failure(APIError(status: 502, text: "Offline.")), .failure(APIError(status: 502, text: "")),
        ])
        offline.dependencies.kept.home = { _ in kept.value }
        await offline.send(.refresh)
        await offline.receive(\.failed) {
            $0.screen = homeScreen
            $0.rows = [row("t1")]
            $0.personas = [coder]
            $0.plugins = [drover]
            ($0.loading, $0.asOf, $0.sync, $0.error) = (false, epoch, .reconnecting, "Offline.")
        }
        await offline.send(.refresh)
        await offline.receive(\.failed) { $0.sync = .offline }
        await clock.advance(by: .seconds(3))
        await offline.receive(\.toasted) { $0.error = nil }
    }

    @Test internal func anIdleHomeAsksNothingForAMinuteAndAnUnchangedAnswerChangesNothing() async {
        let (asked, keptCopies) = (LockIsolated(0), LockIsolated(0))
        let store = store([])
        store.dependencies.threads.home = { _ in
            asked.withValue { $0 += 1 }
            return HomeAnswer(screen: homeScreen, threads: [row("t1")], personas: [coder])
        }
        store.dependencies.kept.keepHome = { _, _ in keptCopies.withValue { $0 += 1 } }
        let (changes, changed) = AsyncStream.makeStream(of: Void.self)
        store.dependencies.threads.changes = { _ in changes }
        let task = await store.send(.task)
        changed.yield()
        await store.receive(\.loaded) {
            ($0.screen, $0.rows, $0.personas, $0.plugins) = (homeScreen, [row("t1")], [coder], [drover])
            $0.loading = false
        }
        await clock.advance(by: .seconds(60))
        #expect(asked.value == 1)
        changed.yield()
        await store.receive(\.loaded)
        #expect(asked.value == 2 && keptCopies.value == 1)
        await task.cancel()
    }
}
