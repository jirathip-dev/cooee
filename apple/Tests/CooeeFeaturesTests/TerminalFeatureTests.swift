import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct TerminalFeatureTests {
    nonisolated private static let shell = TerminalShell(name: "owner-1", owner: true)
    nonisolated private static let agent = TerminalShell(name: "coder", owner: false)
    nonisolated private static let prompt =
        #"{"full":true,"rows":1,"cursor":[0,2],"lines":[{"row":0,"runs":[{"text":"$ "}]}]}"#

    private let calls = Calls()

    private func store(
        _ state: TerminalFeature.State = TerminalFeature.State(server: mini),
        _ dependencies: @escaping (inout DependencyValues) -> Void = { _ in }
    ) -> TestStoreOf<TerminalFeature> {
        let calls = calls
        return TestStore(initialState: state) {
            TerminalFeature()
        } withDependencies: {
            $0.terminal.input = { _, terminal, input in
                switch input {
                case .text(let typed): calls.record("\(terminal) text \(typed.text)")
                case .keys(let keys): calls.record("\(terminal) keys \(keys.joined(separator: " "))")
                }
            }
            dependencies(&$0)
        }
    }

    private func typing(into shell: TerminalShell) -> TerminalFeature.State {
        var state = TerminalFeature.State(server: mini)
        state.shells = [shell]
        state.current = shell
        return state
    }

    private func noShellsLeft(_ store: TestStoreOf<TerminalFeature>) async {
        await store.receive(\.listed) {
            $0.shells = []
            $0.current = nil
            $0.note = "No shells yet: New shell starts one."
        }
    }

    @Test internal func opensTheShellSizesItDrawsItsScreenAndLetsGoWhenItCloses() async throws {
        let update = try JSONDecoder().decode(TerminalUpdate.self, from: Data(Self.prompt.utf8))
        let lists = Queue<[TerminalShell]>([[Self.shell], [Self.shell]])
        let size = TerminalSize(cols: 80, rows: 24)
        let calls = calls
        let store = store {
            $0.terminal.list = { _ in lists.next() ?? [] }
            $0.terminal.resize = { _, terminal, size in calls.record("\(terminal) resize \(size.cols)x\(size.rows)") }
            $0.terminal.watch = { _, _ in
                AsyncThrowingStream {
                    $0.yield(update)
                    $0.finish()
                }
            }
        }
        await store.send(.task(size)) { $0.size = size }
        await store.receive(\.listed) {
            $0.shells = [Self.shell]
            $0.current = Self.shell
        }
        await store.receive(\.update) { $0.screen.apply(update) }
        await noShellsLeft(store)
        #expect(store.state.screen.lines == ["$ ▍"])
        #expect(calls.all == ["owner-1 resize 80x24"])
    }

    @Test internal func typingGoesInTheOrderTypedAndCtrlMakesTheNextLetterAControlKey() async {
        var state = typing(into: Self.shell)
        state.sending = true
        let store = store(state)
        await store.send(.type(Keystrokes("ls"))) { $0.pending = [.text(Keystrokes("ls"))] }
        await store.send(.press("enter")) { $0.pending.append(.keys(["enter"])) }
        await store.send(.key(.ctrl)) { $0.ctrl = true }
        await store.send(.type(Keystrokes("C"))) {
            $0.ctrl = false
            $0.pending.append(.keys(["ctrl+c"]))
        }
        await store.send(.sent) { $0.pending = [.keys(["enter"]), .keys(["ctrl+c"])] }
        await store.receive(\.sent) { $0.pending = [.keys(["ctrl+c"])] }
        await store.receive(\.sent) { $0.pending = [] }
        await store.receive(\.sent) { $0.sending = false }
        #expect(calls.all == ["owner-1 text ls", "owner-1 keys enter", "owner-1 keys ctrl+c"])
    }

    @Test internal func eachKeyOfTheRowGoesToTheShellAsItsKeyOrItsCharacter() async {
        let store = store(typing(into: Self.shell))
        for key in TerminalKey.allCases where key != .ctrl {
            await store.send(.key(key)) { $0.sending = true }
            await store.receive(\.sent) { $0.sending = false }
        }
        #expect(
            calls.all == [
                "owner-1 keys esc", "owner-1 keys tab", "owner-1 keys left", "owner-1 keys up", "owner-1 keys down",
                "owner-1 keys right", "owner-1 text /", "owner-1 text |", "owner-1 text ~", "owner-1 text -",
            ])
    }

    @Test internal func ctrlLatchesForTheNextKeyOnlyThenLetsGo() async {
        var state = typing(into: Self.shell)
        state.sending = true
        let store = store(state)
        await store.send(.key(.ctrl)) { $0.ctrl = true }
        await store.send(.key(.upArrow)) {
            $0.ctrl = false
            $0.pending = [.keys(["up"])]
        }
        await store.send(.key(.ctrl)) { $0.ctrl = true }
        await store.send(.key(.dash)) {
            $0.ctrl = false
            $0.pending.append(.text(Keystrokes("-")))
        }
        await store.send(.key(.ctrl)) { $0.ctrl = true }
        await store.send(.type(Keystrokes("c"))) {
            $0.ctrl = false
            $0.pending.append(.keys(["ctrl+c"]))
        }
        await store.send(.type(Keystrokes("c"))) { $0.pending.append(.text(Keystrokes("c"))) }
        await store.send(.key(.ctrl)) { $0.ctrl = true }
        await store.send(.key(.ctrl)) { $0.ctrl = false }
    }

    @Test internal func anAgentsTerminalIsWatchedButTakesNoKeys() async {
        let store = store(typing(into: Self.agent))
        await store.send(.type(Keystrokes("rm -rf /")))
        await store.send(.press("enter"))
        #expect(calls.all.isEmpty)
    }

    @Test internal func aLockedDeviceIsToldWhyAndHowToGoOn() async {
        let store = store { $0.terminal.list = { _ in throw APIError(status: 403, text: "") } }
        await store.send(.task(nil))
        await store.receive(\.failed) {
            $0.note = "Locked: unlock this device with its passkey first. Your shells keep running."
        }
    }

    @Test internal func aNewShellOpensOnceStarted() async {
        let lists = Queue<[TerminalShell]>([[Self.agent, Self.shell], []])
        let store = store {
            $0.terminal.shell = { _ in "owner-1" }
            $0.terminal.watch = { _, _ in .finished() }
            $0.terminal.list = { _ in lists.next() ?? [] }
        }
        await store.send(.newShell)
        await store.receive(\.listed) {
            $0.shells = [Self.agent, Self.shell]
            $0.current = Self.shell
        }
        await noShellsLeft(store)
    }

    @Test internal func whatIsTypedNeverShowsInAnActionOrStateDump() {
        var state = typing(into: Self.shell)
        state.pending = [.text(Keystrokes("hunter2"))]
        let action = TerminalFeature.Action.type(Keystrokes("hunter2"))
        for dumped in [String(customDumping: action), String(customDumping: state), String(describing: action)] {
            #expect(!dumped.contains("hunter2"), "\(dumped)")
        }
    }
}
