import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

// SAFETY: a JSON literal that decodes.
private let kitchen = try! JSONDecoder().decode(
    Looks.self,
    from: Data(
        #"{"themes":{"kitchen":{"label":"kitchen","scheme":{"dark":"k","light":"kl"},"sprites":true}},"schemes":{}}"#
            .utf8)
)

@MainActor
internal struct MenuFeatureTests {
    private let calls = Calls()

    private func store(_ server: Server? = mini, push: String? = nil) -> TestStoreOf<MenuFeature> {
        let calls = calls
        return TestStore(initialState: MenuFeature.State(server: server, settings: Settings(push: push))) {
            MenuFeature()
        } withDependencies: {
            $0.settingsStore.save = { settings in
                calls.record("save \(settings.scheme) \(settings.style) \(settings.theme) \(settings.push ?? "-")")
            }
            $0.threads.looks = { _ in kitchen }
            $0.push.token = { "ab12" }
            $0.push.register = { _, token, enabled in calls.record("register \(token) \(enabled)") }
            $0.appIcon.set = { icon in calls.record("icon \(icon.asset)") }
        }
    }

    @Test internal func theLookIsPickedAndKept() async {
        let store = store()
        await store.send(.open(true)) { $0.open = true }
        await store.receive(\.looks) { $0.looks = kitchen }
        await store.send(.path([.look])) { $0.path = [.look] }
        await store.send(.scheme(.dark)) { $0.settings.scheme = .dark }
        await store.send(.style(.terminal)) { $0.settings.style = .terminal }
        await store.send(.theme("kitchen")) { $0.settings.theme = "kitchen" }
        await store.send(.open(false)) {
            $0.open = false
            $0.path = []
        }
        #expect(
            calls.all.filter { $0.hasPrefix("save") } == [
                "save dark modern farm -", "save dark terminal farm -", "save dark terminal kitchen -",
            ])
        #expect(calls.all.contains("icon TerminalPrompt"))
    }

    @Test internal func theLooksLoadOnlyWithAServerAndKeepFarmWhenTheReadFails() async {
        await store(nil).send(.load)
        let store = store()
        store.dependencies.threads.looks = { _ in throw APIError(status: 500, text: "") }
        await store.send(.load)
        #expect(store.state.looks == Looks() && store.state.settings.theme == "farm")
    }

    @Test internal func aPickedIconStaysThroughStyleChangesUntilTheLookIsFollowedAgain() async {
        let store = store()
        let koala = IconSet.bush.icons[2]
        await store.send(.icon(koala)) { $0.settings.icon = koala }
        await store.send(.style(.scene)) { $0.settings.style = .scene }
        await store.send(.icon(nil)) { $0.settings.icon = nil }
        await store.send(.icon(IconSet.farm.icons[0])) {
            $0.settings.icon = IconSet.farm.icons[0]
        }
        #expect(
            calls.all == [
                "save system modern farm -", "icon BushKoala", "save system scene farm -", "save system scene farm -",
                "icon FarmBay",
                "save system scene farm -",
            ])
    }

    @Test internal func aRefusedIconChangeSaysWhichTileToTapAgain() async {
        let store = store()
        store.dependencies.appIcon.set = { _ in throw APIError(status: 0, text: "no") }
        await store.send(.style(.terminal)) { $0.settings.style = .terminal }
        await store.receive(\.failed) { $0.error = "Couldn't change the Home Screen icon. Tap Prompt to try again." }
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
        await store(nil).send(.push(true))
    }
}
