import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct ScreenFeatureTests {
    nonisolated internal static let queue = Place(
        plugin: "drover", key: "queue", label: "Queue", screen: "/screens/queue")
    nonisolated internal static let screen: JSONValue = .object([
        "type": .string("screen"), "id": .string("queue"), "title": .string("Queue"),
        "stream": .string("/screens/queue/stream"),
        "blocks": .array([.object(["type": .string("stat"), "id": .string("lanes"), "value": .string("7")])]),
    ])
    nonisolated internal static let merge: JSONValue = .object([
        "seq": .number(1),
        "ops": .array([
            .object(["op": .string("merge"), "id": .string("lanes"), "fields": .object(["value": .string("6")])])
        ]),
    ])

    internal let calls = Calls()
    internal let clock = TestClock()

    internal func store(
        _ dependencies: @escaping (inout DependencyValues) -> Void = { _ in }
    ) -> TestStoreOf<ScreenFeature> {
        let (calls, clock) = (calls, clock)
        return TestStore(initialState: ScreenFeature.State(server: mini, place: Self.queue)) {
            ScreenFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.continuousClock = clock
            $0.screenCache.save = { key, kept in calls.record("keep \(key) \(kept.kept == epoch)") }
            $0.screenCache.backgrounds = { .finished }
            dependencies(&$0)
        }
    }

    nonisolated private static func once() -> AsyncThrowingStream<JSONValue, any Error> {
        AsyncThrowingStream { continuation in
            continuation.yield(merge)
            continuation.finish(throwing: APIError(status: 502, text: ""))
        }
    }

    @Test internal func aScreenLoadsFollowsItsStreamAndIsKept() async {
        let store = store {
            $0.screenCache.load = { _ in nil }
            $0.plugins.screen = { _, plugin, path in
                #expect(plugin == .plugin("drover") && path == "/screens/queue")
                return Self.screen
            }
            $0.plugins.events = { _, _, path in
                #expect(path == "/screens/queue/stream")
                return Self.once()
            }
        }
        let task = await store.send(.task)
        await store.receive(\.loaded) {
            $0.screen = LiveScreen(Self.screen)
            $0.asOf = epoch
        }
        await store.receive(\.event) { $0.screen?.apply(event: Self.merge) }
        await store.receive(\.failed) { $0.unreachable = "drover isn't answering" }
        #expect(store.state.offline && store.state.title == "Queue")
        #expect(store.state.screen?.root["blocks"]?.items.first?.string("value") == "6")
        #expect(calls.all.isEmpty)
        await clock.advance(by: .seconds(2))
        await task.cancel()
        #expect(calls.all == ["keep https://mini.local|drover|/screens/queue true"])
    }

    @Test internal func anUnreachableMacShowsTheKeptAnswerWithItsAgeThenRetries() async {
        let kept = KeptScreen(screen: Self.screen, kept: Date(timeIntervalSince1970: 999_700))
        let answers = Queue<Bool>([false, true])
        let store = store {
            $0.screenCache.load = { _ in kept }
            $0.plugins.screen = { _, _, _ in
                guard answers.next() == true else { throw URLError(.notConnectedToInternet) }
                return .object(["title": .string("Queue")])
            }
        }
        let task = await store.send(.task)
        await store.receive(\.failed) {
            $0.unreachable = "Mac unreachable"
            $0.screen = LiveScreen(Self.screen)
            $0.asOf = kept.kept
        }
        await store.send(.act(BlockAction(action: "drive", values: [:])))
        await clock.advance(by: .seconds(5))
        await store.receive(\.loaded) {
            $0.screen = LiveScreen(.object(["title": .string("Queue")]))
            $0.asOf = epoch
            $0.unreachable = nil
        }
        await clock.advance(by: .seconds(2))
        await task.cancel()
    }

    @Test internal func aScreenWithNoStreamStopsAfterItsAnswer() async {
        let store = store { $0.plugins.screen = { _, _, _ in .object(["title": .string("Hi")]) } }
        await store.send(.retry)
        await store.receive(\.loaded) {
            $0.screen = LiveScreen(.object(["title": .string("Hi")]))
            $0.asOf = epoch
        }
        await clock.advance(by: .seconds(2))
        #expect(store.state.title == "Hi")
    }

    @Test internal func aLaneTapAsksTheChatToOpenItsThread() async {
        let store = store()
        await store.send(.open("herdsman-impl-641"))
        await store.receive(\.delegate.open, "herdsman-impl-641")
    }
}

extension ScreenFeatureTests {
    nonisolated private static func event(_ seq: Int) -> JSONValue {
        .object([
            "seq": .number(Double(seq)),
            "ops": .array([
                .object([
                    "op": .string("merge"), "id": .string("lanes"), "fields": .object(["value": .string("\(seq)")]),
                ])
            ]),
        ])
    }

    @Test internal func aHundredEventsInASecondAreKeptOnceTwoSecondsAfterTheLast() async {
        let store = store()
        await store.send(.loaded(Self.screen)) {
            $0.screen = LiveScreen(Self.screen)
            $0.asOf = epoch
        }
        for seq in 1...100 {
            await clock.advance(by: .milliseconds(10))
            await store.send(.event(Self.event(seq))) { $0.screen?.apply(event: Self.event(seq)) }
        }
        await clock.advance(by: .milliseconds(1_999))
        #expect(calls.all.isEmpty)
        await clock.advance(by: .milliseconds(1))
        #expect(calls.all == ["keep https://mini.local|drover|/screens/queue true"])
    }

    @Test internal func leavingAScreenKeepsItAtOnce() async {
        let store = store { $0.plugins.screen = { _, _, path in .object(["title": .string(path)]) } }
        await store.send(.loaded(Self.screen)) {
            $0.screen = LiveScreen(Self.screen)
            $0.asOf = epoch
        }
        await store.send(.navigate("/screens/global")) {
            $0.path = ["/screens/global"]
            $0.screen = nil
        }
        await store.receive(\.loaded) { $0.screen = LiveScreen(.object(["title": .string("/screens/global")])) }
        await clock.advance(by: .seconds(2))
        #expect(
            calls.all == [
                "keep https://mini.local|drover|/screens/queue true",
                "keep https://mini.local|drover|/screens/global true",
            ])
    }

    @Test internal func goingToTheBackgroundKeepsTheScreenAtOnce() async {
        let (backgrounds, background) = AsyncStream.makeStream(of: Void.self)
        let store = store {
            $0.screenCache.backgrounds = { backgrounds }
            $0.plugins.screen = { _, _, _ in .object(["title": .string("Queue")]) }
        }
        let task = await store.send(.task)
        await store.receive(\.loaded) {
            $0.screen = LiveScreen(.object(["title": .string("Queue")]))
            $0.asOf = epoch
        }
        background.yield()
        await store.receive(\.background)
        await task.cancel()
        await clock.advance(by: .seconds(2))
        #expect(calls.all == ["keep https://mini.local|drover|/screens/queue true"])
    }

    @Test internal func aCoreScreenIsFetchedFromCoreAndKeptUnderCore() async {
        let home = ScreenFeature.State(server: mini, source: .core, path: "/screens/home", label: "Home")
        let (calls, clock) = (calls, clock)
        let store = TestStore(initialState: home) {
            ScreenFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.continuousClock = clock
            $0.screenCache.save = { key, _ in calls.record("keep \(key)") }
            $0.plugins.screen = { _, source, path in
                #expect(source == .core && path == "/screens/home")
                return .object(["title": .string("cooee")])
            }
        }
        await store.send(.retry)
        await store.receive(\.loaded) {
            $0.screen = LiveScreen(.object(["title": .string("cooee")]))
            $0.asOf = epoch
        }
        await clock.advance(by: .seconds(2))
        #expect(calls.all == ["keep https://mini.local|core|/screens/home"])
    }
}
