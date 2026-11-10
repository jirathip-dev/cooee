import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

private func pairCode() throws -> PairCode {
    try #require(
        PairCode(output: "code  123 456  (one use, 10 minutes)\nlink  http://127.0.0.1:4747/pair?code=123456\n"))
}

@MainActor
internal struct MacSetupFeatureTests {
    private let calls = Calls()
    private let clock = TestClock()
    private let items = LoginItems(cooee: .enabled, shed: .needsApproval)
    private let claude = FoundAgent(name: "claude", version: "2.4.1", path: "/opt/homebrew/bin/claude")
    private let home = Network(name: "mini", lan: "mini.local", tailnet: nil)
    private let phone = PairRequest(id: "ab12cd34", word: "maple fox", label: "iPhone")
    private let tablet = PairRequest(id: "ef56ab78", word: "oak hen", label: "iPad")

    private func store(
        _ step: MacStep = .welcome,
        requests: [[PairRequest]] = [],
        _ dependencies: @escaping (inout DependencyValues) -> Void = { _ in }
    ) throws -> TestStoreOf<MacSetupFeature> {
        let mac = try mac(Queue(requests))
        let (calls, clock) = (calls, clock)
        return TestStore(initialState: MacSetupFeature.State(step: step)) {
            MacSetupFeature()
        } withDependencies: {
            $0.continuousClock = clock
            $0.mac = mac
            $0.pairing.pair = { url, code in "tok-\(url.absoluteString)-\(code)" }
            $0.serverStore.save = { calls.record("save \($0.token)") }
            dependencies(&$0)
        }
    }

    private func mac(_ asks: Queue<[PairRequest]>) throws -> MacServerClient {
        let (calls, items, claude, home, code) = (calls, items, claude, home, try pairCode())
        return MacServerClient(
            start: {
                calls.record("start")
                return items
            },
            folders: { ["~/Projects"] },
            addFolder: { calls.record("add \($0)") },
            agents: { [claude] },
            signIns: { [SignIn(provider: "claude", name: "guy", signedIn: true)] },
            signIn: { calls.record("sign in \($0)") },
            network: { home },
            code: { code },
            requests: { asks.next() ?? [] },
            answer: { id, yes in calls.record("\(yes ? "approve" : "decline") \(id)") }
        )
    }

    @Test internal func settingUpStartsTheLoginItemsThenShowsFoldersAndAgents() async throws {
        let store = try store()
        await store.send(.setUp)
        await store.receive(\.started) {
            $0.items = self.items
            $0.step = .folders
        }
        await store.receive(\.foldersLoaded) { $0.folders = ["~/Projects"] }
        await store.receive(\.agentsLoaded) { $0.agents = [self.claude] }
        #expect(store.state.items?.summary == "cooee on · shed waiting for approval in Login Items")
        #expect(calls.all == ["start"])
    }

    @Test internal func theStepsGoInOrderEachLoadingWhatItShows() async throws {
        let store = try store(.folders)
        await store.send(.next) { $0.step = .signins }
        await store.receive(\.signInsLoaded) { $0.signIns = [SignIn(provider: "claude", name: "guy", signedIn: true)] }
        await store.send(.next) { $0.step = .tailscale }
        await store.receive(\.checkNetwork)
        await store.receive(\.networkChecked) { $0.network = self.home }
        await store.send(.back) { $0.step = .signins }
        await store.receive(\.signInsLoaded)
        await store.send(.back) { $0.step = .folders }
        await store.send(.back) { $0.step = .welcome }
        await store.send(.back)
    }

    @Test internal func theQRWaitsForAPhoneAndTheMacApprovesIt() async throws {
        let store = try store(.tailscale, requests: [[], [phone], [phone]])
        await store.send(.next) { $0.step = .pairPhone }
        await store.receive(\.codeShown) { $0.code = try? pairCode() }
        await store.receive(\.requested)
        await clock.advance(by: .seconds(2))
        await store.receive(\.requested) {
            $0.request = self.phone
            $0.step = .approve
        }
        await store.send(.next)
        await store.send(.answer(true)) {
            $0.answered = [self.phone.id]
            $0.request = nil
            $0.step = .pairPhone
            $0.approved = 1
        }
        await clock.advance(by: .seconds(2))
        await store.receive(\.requested)
        await store.send(.back) { $0.step = .tailscale }
        await store.receive(\.checkNetwork)
        await store.receive(\.networkChecked) { $0.network = self.home }
        #expect(calls.all == ["approve ab12cd34"])
    }

    @Test internal func aPhoneThatGivesUpClosesThePrompt() async throws {
        let store = try store(.tailscale, requests: [[phone], []])
        await store.send(.next) { $0.step = .pairPhone }
        await store.receive(\.codeShown) { $0.code = try? pairCode() }
        await store.receive(\.requested) {
            $0.request = self.phone
            $0.step = .approve
        }
        await clock.advance(by: .seconds(2))
        await store.receive(\.requested) {
            $0.request = nil
            $0.step = .pairPhone
        }
        await store.send(.back) { $0.step = .tailscale }
        await store.receive(\.checkNetwork)
        await store.receive(\.networkChecked) { $0.network = self.home }
    }

    @Test internal func aDeclinedPhoneIsNotApprovedAndDonePairsThisMac() async throws {
        let store = try store(.tailscale, requests: [[tablet]])
        await store.send(.next) { $0.step = .pairPhone }
        await store.receive(\.codeShown) { $0.code = try? pairCode() }
        await store.receive(\.requested) {
            $0.request = self.tablet
            $0.step = .approve
        }
        await store.send(.answer(false)) {
            $0.answered = [self.tablet.id]
            $0.request = nil
            $0.step = .pairPhone
        }
        await store.send(.answer(true))
        await store.send(.done)
        let url = try #require(URL(string: "http://127.0.0.1:4747"))
        await store.receive(\.delegate.paired, Server(url: url, token: "tok-http://127.0.0.1:4747-123456"))
        #expect(calls.all == ["decline ef56ab78", "save tok-http://127.0.0.1:4747-123456"])
    }

    @Test internal func foldersSignInsAndTheNetworkCheckRunOnTheMac() async throws {
        let store = try store(.folders)
        await store.send(.addFolder("/home/me/src"))
        await store.receive(\.foldersLoaded) { $0.folders = ["~/Projects"] }
        await store.send(.signIn("codex"))
        await store.send(.checkNetwork)
        await store.receive(\.networkChecked) { $0.network = self.home }
        await store.send(.connectElsewhere)
        await store.receive(\.delegate.clientOnly)
        #expect(calls.all == ["add /home/me/src", "sign in codex"])
    }

    @Test internal func aFailureShowsAndTryingAgainClearsIt() async throws {
        let store = try store {
            $0.mac.start = { throw APIError(status: 1, text: "launchd said no") }
        }
        await store.send(.setUp)
        await store.receive(\.failed) { $0.error = "launchd said no" }
        await store.send(.setUp) { $0.error = nil }
        await store.receive(\.failed) { $0.error = "launchd said no" }
    }
}
