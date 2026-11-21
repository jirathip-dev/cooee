import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

private struct Refused: Error, LocalizedError {
    fileprivate var errorDescription: String? { "The user canceled the sign-in." }
}

private func decoded<T: Decodable>(_ json: String) -> T {
    // SAFETY: every literal below decodes; a typo fails the test that reads it.
    try! JSONDecoder().decode(T.self, from: Data(json.utf8))
}

private let linear: ConnectorList = decoded(
    #"""
    {"catalogue": [{"name": "linear", "label": "Linear", "publisher": "Linear", "source": "catalogue",
      "host": "mcp.linear.app", "leaves": "", "checked": "2026-11-20", "auth": "oauth",
      "tools": {"read": [], "write": [], "never": []}, "accounts": []}], "connectors": []}
    """#)

@MainActor
internal struct ConnectorsFeatureTests {
    private let calls = Calls()

    private func store(start: String, flow: String, sheet: (any Error)? = nil) -> TestStoreOf<ConnectorsFeature> {
        let calls = calls
        let (start, flow): (SignInStart, SignInFlow) = (decoded(start), decoded(flow))
        return TestStore(initialState: ConnectorsFeature.State(server: mini)) {
            ConnectorsFeature()
        } withDependencies: {
            $0.connectors = Self.connectors(calls, start, flow)
            $0.webSignIn.open = { url, scheme in
                calls.record("open \(url.host() ?? "") until \(scheme)")
                if let sheet { throw sheet }
            }
            $0.threads.personas = { _ in [coder] }
            $0.plugins.act = { _, _, screen, action in
                let picks = action.values["mcp"]?.items.map {
                    "\($0.string("server") ?? "")/\($0.string("account") ?? "")"
                }
                calls.record("save \(screen) \(picks ?? [])")
                return .null
            }
        }
    }

    private static func connectors(_ calls: Calls, _ start: SignInStart, _ flow: SignInFlow) -> ConnectorsClient {
        var client = ConnectorsClient()
        client.list = { _ in linear }
        client.add = { _, name in calls.record("add \(name)") }
        client.signIn = { _, name, account in
            calls.record("signin \(name) \(account)")
            return start
        }
        client.flow = { _, id in
            calls.record("flow \(id)")
            return flow
        }
        client.paste = { _, name, id, token in calls.record("paste \(name) \(id) \(token)") }
        return client
    }

    @Test internal func anEntryIsAddedSignedInOnTheServicesPageAndAttachedToAPersonaInOneTap() async {
        let flow = #"{"state": "signed_in", "connector": "linear", "account": "main", "why": null}"#
        let store = store(
            start: #"{"flow": "f1", "open": "https://linear.app/authorize", "until": "cooee"}"#, flow: flow)
        await store.send(.connect("linear", account: "main")) {
            $0.signingIn = MCPPick(server: "linear", account: "main")
        }
        await store.receive(\.ended) {
            $0.signingIn = nil
            $0.connected = decoded(flow)
        }
        await store.receive(\.loaded) { $0.list = linear }
        await store.receive(\.personas) { $0.personas = [coder] }
        await store.send(.attach(persona: "coder")) { $0.connected = nil }
        await store.receive(\.loaded)
        await store.receive(\.personas)
        #expect(
            calls.all == [
                "add linear", "signin linear main", "open linear.app until cooee", "flow f1",
                "save /screens/persona/coder [\"linear/main\"]",
            ])
    }

    @Test internal func theTokenCaseAsksForAPasteAndSendsItToTheMac() async {
        let flow = #"{"state": "signed_in", "connector": "linear", "account": "main", "why": null}"#
        let store = store(start: #"{"flow": "f2", "help": "Make a token."}"#, flow: flow)
        await store.send(.connect("linear", account: "main")) {
            $0.signingIn = MCPPick(server: "linear", account: "main")
        }
        let ask = ConnectorsFeature.TokenAsk(connector: "linear", account: "main", flow: "f2", help: "Make a token.")
        await store.receive(\.ask) {
            $0.signingIn = nil
            $0.token = ask
        }
        await store.send(.paste)
        await store.send(.typed("lin_x")) { $0.token?.value = "lin_x" }
        await store.send(.paste)
        await store.receive(\.ended) {
            $0.token = nil
            $0.connected = decoded(flow)
        }
        await store.receive(\.loaded) { $0.list = linear }
        await store.receive(\.personas) { $0.personas = [coder] }
        await store.send(.notNow) { $0.connected = nil }
        #expect(calls.all == ["add linear", "signin linear main", "paste linear f2 lin_x", "flow f2"])
    }

    @Test internal func aClosedSheetOrARefusedSignInSaysWhyAndLeavesNoAccountSigningIn() async {
        let failed = #"{"state": "failed", "connector": "linear", "account": "main", "why": "the code was refused"}"#
        let store = store(start: #"{"flow": "f3", "open": "https://linear.app/a", "until": "cooee"}"#, flow: failed)
        await store.send(.connect("linear", account: "main")) {
            $0.signingIn = MCPPick(server: "linear", account: "main")
        }
        await store.receive(\.ended) {
            $0.signingIn = nil
            $0.error = "the code was refused"
        }
        await store.receive(\.loaded) { $0.list = linear }
        await store.receive(\.personas) { $0.personas = [coder] }
        store.dependencies.webSignIn.open = { _, _ in throw Refused() }
        await store.send(.connect("linear", account: "main")) {
            $0.signingIn = MCPPick(server: "linear", account: "main")
            $0.error = nil
        }
        await store.receive(\.failed) {
            $0.signingIn = nil
            $0.error = "The user canceled the sign-in."
        }
        await store.receive(\.loaded)
        await store.receive(\.personas)
    }
}

@MainActor
internal struct ConnectorsPageTests {
    @Test internal func openingSettingsConnectorsLoadsTheListAndTheMenuKeepsNoOtherSave() async {
        let calls = Calls()
        let store = TestStore(initialState: MenuFeature.State(server: mini, settings: Settings())) {
            MenuFeature()
        } withDependencies: {
            $0.connectors.list = { _ in linear }
            $0.threads.personas = { _ in [coder] }
            $0.settingsStore.save = { _ in calls.record("settings saved") }
        }
        await store.send(.path([.connectors])) {
            $0.path = [.connectors]
            $0.connectors = ConnectorsFeature.State(server: mini)
        }
        await store.receive(\.connectors.task)
        await store.receive(\.connectors.loaded) { $0.connectors?.list = linear }
        await store.receive(\.connectors.personas) { $0.connectors?.personas = [coder] }
        await store.send(.connectors(.open("linear"))) { $0.connectors?.entry = "linear" }
        #expect(calls.all.isEmpty)
    }
}
