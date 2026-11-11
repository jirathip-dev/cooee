import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct MenuFeatureTests {
    private let calls = Calls()

    private func store(_ server: Server? = mini, push: String? = nil) -> TestStoreOf<MenuFeature> {
        let calls = calls
        return TestStore(initialState: MenuFeature.State(server: server, settings: Settings(push: push))) {
            MenuFeature()
        } withDependencies: {
            $0.settingsStore.save = { settings in
                calls.record("save \(settings.scheme) \(settings.style) \(settings.push ?? "-")")
            }
            $0.push.token = { "ab12" }
            $0.push.register = { _, token, enabled in calls.record("register \(token) \(enabled)") }
        }
    }

    @Test internal func theLookIsPickedAndKept() async {
        let store = store()
        await store.send(.open(true)) { $0.open = true }
        await store.send(.path([.look])) { $0.path = [.look] }
        await store.send(.scheme(.dark)) { $0.settings.scheme = .dark }
        await store.send(.style(.terminal)) { $0.settings.style = .terminal }
        await store.send(.open(false)) {
            $0.open = false
            $0.path = []
        }
        #expect(calls.all == ["save dark modern -", "save dark terminal -"])
    }

    @Test internal func pushOnRegistersTheDeviceTokenAndOffRemovesIt() async {
        let store = store()
        await store.send(.push(true)) { $0.busy = true }
        await store.receive(\.pushed) {
            $0.busy = false
            $0.settings.push = "ab12"
        }
        await store.send(.push(true))
        await store.send(.push(false)) { $0.busy = true }
        await store.receive(\.pushed) {
            $0.busy = false
            $0.settings.push = nil
        }
        #expect(
            calls.all == [
                "register ab12 true", "save system modern ab12", "register ab12 false", "save system modern -",
            ])
    }

    @Test internal func aRefusedPushStaysOffWithTheReason() async {
        let store = store()
        store.dependencies.push.token = { throw APIError(status: 0, text: "Notifications are off.") }
        await store.send(.push(true)) { $0.busy = true }
        await store.receive(\.failed) {
            $0.busy = false
            $0.error = "Notifications are off."
        }
        #expect(calls.all.isEmpty)
    }

    @Test internal func withoutAServerPushDoesNothing() async {
        await store(nil).send(.push(true))
    }
}
