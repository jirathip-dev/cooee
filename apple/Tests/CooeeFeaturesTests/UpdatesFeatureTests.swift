import ComposableArchitecture
@testable import CooeeFeatures
import Testing

@MainActor
internal struct UpdatesFeatureTests {
    private let calls = Calls()

    private func store(_ status: UpdateStatus) -> TestStoreOf<UpdatesFeature> {
        let calls = calls
        return TestStore(initialState: UpdatesFeature.State(status: status)) {
            UpdatesFeature()
        } withDependencies: {
            $0.updater.check = { calls.record("check") }
            $0.updater.setAutomatic = { calls.record("automatic \($0)") }
        }
    }

    @Test internal func checksAndTurnsAutomaticUpdatesOff() async {
        let store = store(.signed(automatic: true))
        await store.send(.check)
        await store.send(.setAutomatic(false)) { $0.status = .signed(automatic: false) }
        await store.finish()
        #expect(calls.all == ["check", "automatic false"])
    }

    @Test internal func anUnsignedBuildNeverChecks() async {
        let store = store(.unsigned)
        await store.send(.check)
        await store.send(.setAutomatic(true))
        await store.finish()
        #expect(calls.all.isEmpty)
    }
}
