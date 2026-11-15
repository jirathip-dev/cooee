import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct LayoutFeatureTests {
    nonisolated private static let home: JSONValue = .object([
        "format": .number(1),
        "root": .object([
            "type": .string("nav"),
            "slots": .object([
                "main": .array([
                    .object([
                        "type": .string("screen"), "source": .string("core"), "path": .string("/screens/home"),
                        "id": .string("home"), "place": .string("push"),
                    ])
                ])
            ]),
        ]),
    ])
    nonisolated private static let good = LayoutAnswer(etag: "e1", layout: home)
    nonisolated private static let bad = LayoutAnswer(
        etag: "e2", layout: .object(["format": .number(1), "root": .object(["type": .string("float")])]))

    private let calls = Calls()

    private func store(
        _ state: LayoutFeature.State = LayoutFeature.State(server: mini, layout: testLayout),
        _ dependencies: @escaping (inout DependencyValues) -> Void = { _ in }
    ) -> TestStoreOf<LayoutFeature> {
        let calls = calls
        return TestStore(initialState: state) {
            LayoutFeature()
        } withDependencies: {
            $0.layoutStore.save = { calls.record("save \($0?.etag ?? "nil")") }
            $0.layoutClient.events = { _ in .finished() }
            dependencies(&$0)
        }
    }

    @Test internal func aFetchedLayoutIsDrawnAndKept() async throws {
        let store = store {
            $0.layoutClient.fetch = { _, etag in
                #expect(etag == "")
                return Self.good
            }
        }
        let drawn = layout(#"{"format":1,"root":{"type":"nav","slots":{"main":[\#(Self.mainHome)]}}}"#, etag: "e1")
        await store.send(.task)
        await store.receive(\.fetched) { $0.layout = drawn }
        #expect(store.state.layout.resolved(for: .compact).slots["main"]?.map(\.id) == ["home", "pairing"])
        #expect(calls.all == ["save e1"])
    }

    @Test internal func aLayoutTheAppCantReadKeepsTheLastGoodAndSaysWhy() async {
        let store = store { $0.layoutClient.fetch = { _, _ in Self.bad } }
        await store.send(.task)
        await store.receive(\.fetched) {
            $0.broken = #"app can't read this layout: unknownType(path: "root", type: "float") · last good layout kept"#
        }
        #expect(store.state.layout == testLayout && calls.all.isEmpty)
    }

    @Test internal func aBrokenFileOnTheMacKeepsTheLastGoodUntilAGoodWrite() async {
        let events = AsyncThrowingStream<LayoutEvent, any Error> { continuation in
            continuation.yield(.broken(line: 12, why: #"unknown place "float""#))
            continuation.yield(.changed(etag: "e1"))
            continuation.finish()
        }
        let fetches = Queue<LayoutAnswer?>([nil, Self.good])
        let store = store {
            $0.layoutClient.fetch = { _, _ in fetches.next() ?? nil }
            $0.layoutClient.events = { _ in events }
        }
        await store.send(.task)
        await store.receive(\.event) {
            $0.broken = #"layout.json line 12: unknown place "float" · last good layout kept"#
        }
        #expect(store.state.layout == testLayout)
        await store.receive(\.event) { $0.broken = nil }
        await store.receive(\.fetched) { $0.layout = layout(#"{"format":1,"root":\#(Self.navHome)}"#, etag: "e1") }
    }

    @Test internal func aChangeToTheLayoutAlreadyDrawnFetchesNothing() async {
        var state = LayoutFeature.State(
            server: mini, layout: layout(#"{"format":1,"root":{"type":"nav"}}"#, etag: "e1"))
        state.broken = "layout.json line 3: bad"
        let store = store(state)
        await store.send(.event(.changed(etag: "e1"))) { $0.broken = nil }
    }

    @Test internal func resetAppliesTheMacsAnswer() async {
        let store = store { $0.layoutClient.reset = { _ in Self.good } }
        await store.send(.reset)
        await store.receive(\.resetDone) { $0.layout = layout(#"{"format":1,"root":\#(Self.navHome)}"#, etag: "e1") }
        #expect(calls.all == ["save e1"])
    }

    @Test internal func resetOfflineDrawsTheBundledDefaultAndForgetsTheLastGood() async throws {
        let file = FileManager.default.temporaryDirectory.appending(path: "layout-\(UUID().uuidString).json")
        let disk = LayoutFile(file: file)
        disk.save(Self.good)
        let bundled = layout(#"{"format":1,"theme":"farm","root":{"type":"nav"}}"#)
        let kept = LayoutStore.onDisk(disk, bundled: bundled)
        let last = try kept.load()
        let store = TestStore(initialState: LayoutFeature.State(server: mini, layout: last)) {
            LayoutFeature()
        } withDependencies: {
            $0.layoutStore = kept
            $0.layoutClient.reset = { _ in throw URLError(.notConnectedToInternet) }
        }
        #expect(last.etag == "e1")
        await store.send(.reset)
        await store.receive(\.resetDone) {
            $0.layout = bundled
            $0.broken = "reset here; the Mac's layout.json is unchanged until it's back"
        }
        #expect(try disk.load() == nil && kept.load() == bundled)
    }

    nonisolated private static let mainHome =
        #"{"type":"screen","source":"core","path":"/screens/home","id":"home","place":"push"}"#
    nonisolated private static let navHome = #"{"type":"nav","slots":{"main":[\#(mainHome)]}}"#
}
