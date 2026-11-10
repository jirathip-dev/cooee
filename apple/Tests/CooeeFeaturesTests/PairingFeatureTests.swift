import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct PairingFeatureTests {
    private let calls = Calls()
    private let clock = TestClock()

    private func store(
        claims: [PairClaim] = [],
        _ dependencies: @escaping (inout DependencyValues) -> Void = { _ in }
    ) -> TestStoreOf<PairingFeature> {
        let answers = Queue(claims)
        let (calls, clock) = (calls, clock)
        return TestStore(initialState: PairingFeature.State()) {
            PairingFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.continuousClock = clock
            $0.pairing.pair = { url, code in "tok-\(url.host() ?? "")-\(code)" }
            $0.pairing.ask = { _ in PairAsk(word: "maple", claim: "cl1") }
            $0.pairing.claim = { _, _ in answers.next() ?? .waiting }
            $0.pairing.cancel = { _, claim in calls.record("cancel \(claim)") }
            $0.serverStore.save = { calls.record("save \($0.token)") }
            $0.threads.personas = { _ in [coder] }
            dependencies(&$0)
        }
    }

    @Test internal func aLinkWithACodePairsAndGoesHome() async throws {
        let store = store()
        let url = try #require(URL(string: "http://127.0.0.1:4100"))
        let server = Server(url: url, token: "tok-127.0.0.1-123456")
        await store.send(.connect("http://127.0.0.1:4100/pair?code=123456")) {
            $0.url = url
            $0.host = "127.0.0.1"
        }
        await store.receive(\.paired) {
            $0.server = server
            $0.step = .paired
        }
        await store.receive(\.personasLoaded) { $0.personas = [coder] }
        await store.send(.home)
        await store.receive(\.delegate.paired)
        #expect(calls.all == ["save tok-127.0.0.1-123456"])
    }

    @Test internal func anAddressAsksThenWaitsForApproval() async throws {
        let store = store(claims: [.waiting, .paired("t2")])
        let url = try #require(URL(string: "https://mini.local"))
        await store.send(.connect("mini.local")) {
            $0.url = url
            $0.host = "mini.local"
        }
        await store.receive(\.asked) {
            $0.word = "maple"
            $0.until = epoch.addingTimeInterval(300)
            $0.claim = "cl1"
            $0.step = .waiting
        }
        await clock.advance(by: .seconds(2))
        await store.receive(\.claimed) {
            $0.server = Server(url: url, token: "t2")
            $0.claim = nil
            $0.step = .paired
        }
        await store.receive(\.personasLoaded) { $0.personas = [coder] }
    }

    @Test internal func aDeclineGoesBackToTheAddressWithWhy() async {
        let store = store(claims: [.declined("mini said no.")])
        await store.send(.connect("mini.local")) {
            $0.url = URL(string: "https://mini.local")
            $0.host = "mini.local"
        }
        await store.receive(\.asked) {
            $0.word = "maple"
            $0.until = epoch.addingTimeInterval(300)
            $0.claim = "cl1"
            $0.step = .waiting
        }
        await store.receive(\.claimed) {
            $0.error = "mini said no."
            $0.claim = nil
            $0.step = .address
        }
    }

    @Test internal func aBadAddressOrAFailedAskSaysWhy() async {
        let store = store {
            $0.pairing.ask = { _ in throw APIError(status: 500, text: "Locked.") }
        }
        await store.send(.connect("  ")) { $0.error = "Type an address, like server.example.com." }
        await store.send(.connect("mini.local")) {
            $0.error = nil
            $0.url = URL(string: "https://mini.local")
            $0.host = "mini.local"
        }
        await store.receive(\.failed) { $0.error = "Locked." }
    }

    @Test internal func cancelTakesBackTheAskAndGoesToTheWelcome() async {
        let store = store()
        await store.send(.connect("mini.local")) {
            $0.url = URL(string: "https://mini.local")
            $0.host = "mini.local"
        }
        await store.receive(\.asked) {
            $0.word = "maple"
            $0.until = epoch.addingTimeInterval(300)
            $0.claim = "cl1"
            $0.step = .waiting
        }
        await store.send(.cancel) {
            $0.claim = nil
            $0.step = .welcome
        }
        await store.finish()
        #expect(calls.all == ["cancel cl1"])
        await store.send(.cancel)
    }

    @Test internal func stepsTheAddressAndNearbyServers() async {
        let found = [NearbyServer(name: "mini", address: "http://192.0.2.20:4100")]
        let store = store {
            $0.nearby.browse = {
                AsyncStream {
                    $0.yield(found)
                    $0.finish()
                }
            }
        }
        await store.send(.show(.address)) { $0.step = .address }
        await store.send(.addressChanged("mini")) { $0.address = "mini" }
        await store.send(.browse) { $0.browsing = true }
        await store.receive(\.found) { $0.nearby = found }
        await store.send(.browse)
        await store.send(.home)
        await store.send(.claimed(.waiting))
        await store.send(.claimed(.paired("t")))
        await store.send(.asked(PairAsk(word: "w", claim: "c")))
        await store.send(.failed("no")) { $0.error = "no" }
        await store.send(.show(.welcome)) {
            $0.step = .welcome
            $0.error = nil
        }
    }
}
