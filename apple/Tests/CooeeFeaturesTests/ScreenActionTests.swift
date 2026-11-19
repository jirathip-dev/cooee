import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Testing

extension ScreenFeatureTests {
    @Test internal func anActionAppliesItsAnswersOpsAndSaysItsToast() async {
        let answer: JSONValue = .object(["toast": .string("Driving"), "ops": Self.merge["ops"] ?? .null])
        var state = ScreenFeature.State(server: mini, place: Self.queue)
        state.screen = LiveScreen(Self.screen)
        let store = TestStore(initialState: state) {
            ScreenFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.continuousClock = clock
            $0.screenCache.save = { _, _ in }
            $0.plugins.act = { _, plugin, screen, action in
                #expect(plugin == .plugin("drover") && screen == "queue" && action.action == "drive")
                return answer
            }
        }
        await store.send(.act(BlockAction(action: "drive", values: [:]))) {
            $0.pending = [BlockAction(action: "drive", values: [:])]
        }
        await store.receive(\.acted) {
            $0.pending = []
            $0.said = "Driving"
            $0.screen?.apply(event: answer)
            $0.asOf = epoch
        }
        await clock.advance(by: .seconds(2))
    }

    @Test internal func aRefusedActionSaysWhy() async {
        let store = store { $0.plugins.act = { _, _, _, _ in throw APIError(status: 403, text: "drop isn't allowed") } }
        await store.send(.act(BlockAction(action: "drop", values: [:]))) {
            $0.pending = [BlockAction(action: "drop", values: [:])]
        }
        await store.receive(\.acted) {
            $0.pending = []
            $0.said = "drop isn't allowed"
        }
    }

    @Test internal func aTapDuringAnActionIsPendingAtOnceThenSentInOrder() async {
        let (drive, drop) = (BlockAction(action: "drive", values: [:]), BlockAction(action: "drop", values: [:]))
        let (calls, clock) = (calls, clock)
        let store = store {
            $0.plugins.act = { _, _, _, action in
                calls.record("send \(action.action)")
                try await clock.sleep(for: .seconds(4))
                return .object(["toast": .string(action.action)])
            }
        }
        await store.send(.act(drive)) { $0.pending = [drive] }
        await store.send(.act(drop)) { $0.pending = [drive, drop] }
        await store.send(.act(drop))
        await clock.advance(by: .seconds(4))
        await store.receive(\.acted) {
            $0.pending = [drop]
            $0.said = "drive"
            $0.asOf = epoch
        }
        await clock.advance(by: .seconds(6))
        await store.receive(\.acted) {
            $0.pending = []
            $0.said = "drop"
            $0.asOf = epoch
        }
        #expect(calls.all == ["send drive", "send drop"])
    }
}
