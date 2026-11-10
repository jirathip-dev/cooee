import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct PluginScreenFeatureTests {
    private static let queue = Place(plugin: "drover", key: "queue", label: "Queue", screen: "/screens/queue")
    private static let screen: JSONValue = .object([
        "type": .string("screen"), "id": .string("queue"), "title": .string("Queue"),
        "stream": .string("/screens/queue/stream"),
        "blocks": .array([.object(["type": .string("stat"), "id": .string("lanes"), "value": .string("7")])]),
    ])
    nonisolated private static let merge: JSONValue = .object([
        "seq": .number(1),
        "ops": .array([
            .object(["op": .string("merge"), "id": .string("lanes"), "fields": .object(["value": .string("6")])])
        ]),
    ])

    private let calls = Calls()
    private let clock = TestClock()

    private func store(
        _ dependencies: @escaping (inout DependencyValues) -> Void = { _ in }
    ) -> TestStoreOf<PluginScreenFeature> {
        let (calls, clock) = (calls, clock)
        return TestStore(initialState: PluginScreenFeature.State(server: mini, place: Self.queue)) {
            PluginScreenFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.continuousClock = clock
            $0.screenCache.save = { key, kept in calls.record("keep \(key) \(kept.kept == epoch)") }
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
            $0.plugins.screen = { _, plugin, path in
                #expect(plugin == "drover" && path == "/screens/queue")
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
        await task.cancel()
        #expect(
            calls.all == [
                "keep https://mini.local|drover|/screens/queue true",
                "keep https://mini.local|drover|/screens/queue true",
            ])
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
        await task.cancel()
    }

    @Test internal func anActionAppliesItsAnswersOpsAndSaysItsToast() async {
        let answer: JSONValue = .object(["toast": .string("Driving"), "ops": Self.merge["ops"] ?? .null])
        var state = PluginScreenFeature.State(server: mini, place: Self.queue)
        state.screen = LiveScreen(Self.screen)
        let store = TestStore(initialState: state) {
            PluginScreenFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.screenCache.save = { _, _ in }
            $0.plugins.act = { _, plugin, screen, action in
                #expect(plugin == "drover" && screen == "queue" && action.action == "drive")
                return answer
            }
        }
        await store.send(.act(BlockAction(action: "drive", values: [:]))) { $0.acting = true }
        await store.receive(\.acted) {
            $0.acting = false
            $0.said = "Driving"
            $0.screen?.apply(event: answer)
            $0.asOf = epoch
        }
    }

    @Test internal func aRefusedActionSaysWhy() async {
        let store = store { $0.plugins.act = { _, _, _, _ in throw APIError(status: 403, text: "drop isn't allowed") } }
        await store.send(.act(BlockAction(action: "drop", values: [:]))) { $0.acting = true }
        await store.receive(\.acted) {
            $0.acting = false
            $0.said = "drop isn't allowed"
        }
    }

    @Test internal func aScreenWithNoStreamStopsAfterItsAnswer() async {
        let store = store { $0.plugins.screen = { _, _, _ in .object(["title": .string("Hi")]) } }
        await store.send(.retry)
        await store.receive(\.loaded) {
            $0.screen = LiveScreen(.object(["title": .string("Hi")]))
            $0.asOf = epoch
        }
        #expect(store.state.title == "Hi")
    }
}
