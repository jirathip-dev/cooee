import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct MenuPushTests {
    private let calls = Calls()

    private func store(_ server: Server? = mini, push: String? = nil) -> TestStoreOf<MenuFeature> {
        let calls = calls
        return TestStore(initialState: MenuFeature.State(server: server, settings: Settings(push: push))) {
            MenuFeature()
        } withDependencies: {
            $0.settingsStore.save = { calls.record("save \($0.scheme) \($0.style.name) \($0.theme) \($0.push ?? "-")") }
            $0.push.token = { "ab12" }
            $0.push.register = { _, token, enabled in calls.record("register \(token) \(enabled)") }
            $0.push.registered = { _ in [] }
            $0.push.test = { _ in calls.record("test") }
        }
    }

    @Test internal func pushOnRegistersTheDeviceTokenAndOffRemovesIt() async {
        let store = store()
        await store.send(.push(true)) { $0.busy = true }
        await store.receive(\.pushed) {
            $0.busy = false
            $0.alerts = true
            $0.settings.push = "ab12"
        }
        await store.send(.push(true))
        await store.send(.push(false)) { $0.busy = true }
        await store.receive(\.pushed) {
            $0.busy = false
            $0.alerts = false
            $0.settings.push = nil
        }
        #expect(
            calls.all == [
                "register ab12 true", "save system modern farm ab12", "register ab12 false",
                "save system modern farm -",
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
        let store = store(nil)
        await store.send(.push(true))
        await store.send(.test)
        await store.send(.path([.notifications])) { $0.path = [.notifications] }
    }

    @Test internal func theNotificationsPageIsOffWhenTheServerHasNoTokenForThisDevice() async {
        let store = store(push: "ab12")
        #expect(store.state.alerts)
        await store.send(.path([.notifications])) { $0.path = [.notifications] }
        await store.receive(\.alerts) { $0.alerts = false }
        await store.send(.push(true)) { $0.busy = true }
        await store.receive(\.pushed) {
            $0.busy = false
            $0.alerts = true
        }
        #expect(calls.all == ["register ab12 true", "save system modern farm ab12"])
    }

    @Test internal func theNotificationsPageIsOnWhenTheServerHoldsThisDevicesToken() async {
        let store = store(push: "AB12")
        store.dependencies.push.registered = { _ in ["ab12"] }
        await store.send(.path([.notifications])) { $0.path = [.notifications] }
        await store.receive(\.alerts)
        await store.send(.push(false)) { $0.busy = true }
        await store.receive(\.pushed) {
            $0.busy = false
            $0.alerts = false
            $0.settings.push = nil
        }
        #expect(calls.all == ["register AB12 false", "save system modern farm -"])
    }

    @Test internal func aTestPushIsSentAndItsRefusalShown() async {
        let store = store(push: "ab12")
        await store.send(.test) { $0.busy = true }
        await store.receive(\.tested) { $0.busy = false }
        store.dependencies.push.test = { _ in throw APIError(status: 400, text: "Alerts are off on this device.") }
        await store.send(.test) { $0.busy = true }
        await store.receive(\.failed) {
            $0.busy = false
            $0.error = "Alerts are off on this device."
        }
        #expect(calls.all == ["test"])
    }
}
