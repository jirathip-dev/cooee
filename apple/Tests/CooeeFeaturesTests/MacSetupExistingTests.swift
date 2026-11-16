import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct MacSetupExistingTests {
    private let calls = Calls()
    private let items = LoginItems(cooee: .enabled, shed: .enabled)
    private let cli = ExistingInstall.cli(labels: ["dev.cooee.cooee", "dev.shed.serve"])

    private func store(
        _ found: ExistingInstall, _ dependencies: @escaping (inout DependencyValues) -> Void = { _ in }
    ) throws -> TestStoreOf<MacSetupFeature> {
        let (calls, items) = (calls, items)
        let code = try #require(PairCode(output: "link  http://127.0.0.1:4747/pair?code=123456\n"))
        return TestStore(initialState: MacSetupFeature.State()) {
            MacSetupFeature()
        } withDependencies: {
            $0.mac.existing = { found }
            $0.mac.useExisting = { calls.record("use existing") }
            $0.mac.takeOver = { calls.record("boot out \($0.joined(separator: " "))") }
            $0.mac.start = {
                calls.record("start")
                return items
            }
            $0.mac.folders = { ["~/Projects"] }
            $0.mac.agents = { [] }
            $0.mac.code = { code }
            $0.pairing.pair = { _, code in "tok-\(code)" }
            $0.serverStore.save = { calls.record("save \($0.token)") }
            dependencies(&$0)
        }
    }

    @Test internal func usingTheCLIServerPairsWithItAndRegistersNoLoginItems() async throws {
        let store = try store(cli)
        await store.send(.look)
        await store.receive(\.found) { $0.found = self.cli }
        await store.send(.useExisting)
        await store.receive(\.done)
        let url = try #require(URL(string: "http://127.0.0.1:4747"))
        await store.receive(\.delegate.paired, Server(url: url, token: "tok-123456"))
        #expect(calls.all == ["use existing", "save tok-123456"])
    }

    @Test internal func takingItOverBootsOutTheCLIAgentsThenRegistersTheAppsOwn() async throws {
        let both = ExistingInstall.both(labels: ["dev.cooee.cooee"])
        let store = try store(both)
        await store.send(.look)
        await store.receive(\.found) { $0.found = both }
        await store.send(.takeOver)
        await store.receive(\.setUp)
        await store.receive(\.started) {
            $0.items = self.items
            $0.step = .folders
        }
        await store.receive(\.foldersLoaded) { $0.folders = ["~/Projects"] }
        await store.receive(\.agentsLoaded)
        #expect(calls.all == ["boot out dev.cooee.cooee", "start"])
    }

    @Test internal func aPortTakenByAnotherProgramSaysSo() async throws {
        let store = try store(.portTaken(port: 4747, program: "node"))
        await store.send(.look)
        await store.receive(\.found) {
            $0.found = .portTaken(port: 4747, program: "node")
            $0.error = "Port 4747 is taken by node: stop it, or set another port in config.toml."
        }
        let none = try self.store(.none)
        await none.send(.look)
        await none.receive(\.found) { $0.found = ExistingInstall.none }
        #expect(calls.all.isEmpty)
    }

    @Test internal func aFailedTakeOverOrUseStopsBeforeAnythingStarts() async throws {
        let store = try store(cli) {
            $0.mac.takeOver = { _ in throw APIError(status: 5, text: "bootout failed") }
            $0.mac.useExisting = { throw APIError(status: 1, text: "unregister failed") }
        }
        await store.send(.look)
        await store.receive(\.found) { $0.found = self.cli }
        await store.send(.takeOver)
        await store.receive(\.failed) { $0.error = "bootout failed" }
        await store.send(.useExisting)
        await store.receive(\.failed) { $0.error = "unregister failed" }
        #expect(calls.all.isEmpty)
    }
}
