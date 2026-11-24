import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct ChatMachineTests {
    private let calls = Calls()
    private let machines = [
        MachineRow(name: "", host: "mini"), MachineRow(name: "box", host: "box", error: "agent not signed in"),
        MachineRow(name: "d1", host: "d1", desk: true, provider: "cloud"),
    ]

    private func store(_ target: ChatFeature.Target, on machine: String = "box") -> TestStoreOf<ChatFeature> {
        var state = ChatFeature.State(server: mini, target: target, persona: Persona(coder))
        state.machine.current = machine
        let (calls, machines, refusals) = (calls, machines, Queue([APIError(status: 400, text: "mini isn't ready")]))
        return TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.continuousClock = ImmediateClock()
            $0.personas.machines = { _ in machines }
            $0.threads.move = { _, thread, machine in
                if let refused = refusals.next() { throw refused }
                calls.record("move \(thread) \(machine)")
            }
            $0.threads.start = { _, persona, machine, text, _ in
                calls.record("start \(persona.id) \(machine) \(text)")
                return "n1"
            }
        }
    }

    @Test internal func theChipShowsTheThreadsMachineAndAMoveWaitsUntilItIsReady() async {
        let store = store(.thread("a1"))
        #expect(store.state.machine.shown == nil && store.state.target.started)
        await store.send(.machine(.load))
        await store.receive(\.machine.loaded) { $0.machine.machines = machines }
        #expect(store.state.machine.shown?.dot == .warning)
        #expect(store.state.machine.shown?.problems == ["agent not signed in"])
        await store.send(.machine(.show(true))) { $0.machine.open = true }
        await store.send(.machine(.pick("box")))
        await store.send(.machine(.pick(""))) { $0.machine.moving = true }
        await store.receive(\.machine.refused) {
            ($0.machine.moving, $0.machine.refusal) = (false, "mini isn't ready")
        }
        await store.send(.machine(.pick(""))) { ($0.machine.moving, $0.machine.refusal) = (true, nil) }
        await store.receive(\.machine.moved) {
            ($0.machine.current, $0.machine.moving, $0.machine.open) = ("", false, false)
        }
        #expect(store.state.machine.shown == machines[0])
        #expect(calls.all == ["move a1 "])
    }

    @Test internal func aNewChatsPickIsItsMachineOverrideAndNothingMoves() async {
        let store = store(.new(coder))
        #expect(!store.state.target.started)
        await store.send(.machine(.show(true))) { $0.machine.open = true }
        await store.send(.machine(.pick("d1"))) { ($0.machine.current, $0.machine.open) = ("d1", false) }
        await store.send(.compose(.draftChanged("hi"))) { $0.draft = "hi" }
        await store.send(.compose(.send)) {
            $0.draft = ""
            $0.outbox = [Outgoing(id: -1, text: "hi")]
        }
        await store.receive(\.outbox.delivered) { $0.outbox = [] }
        await store.receive(\.delegate.started)
        #expect(calls.all == ["start coder d1 hi"])
    }

    @Test internal func aMachineGoneFromTheListShowsOfflineAndARoomHasNoChip() async {
        var store = store(.thread("a1"), on: "studio")
        await store.send(.machine(.load))
        await store.receive(\.machine.loaded) { $0.machine.machines = machines }
        #expect(store.state.machine.shown?.dot == .offline)
        #expect(store.state.machine.shown?.problems == ["shed can't reach studio"])
        store = self.store(.thread("r1"))
        store.dependencies.personas.machines = { _ in throw APIError(status: 400, text: "no") }
        await store.send(.machine(.load))
        await store.receive(\.failed) { $0.error = "no" }
        await store.receive(\.link.toasted) { $0.error = nil }
        var room = ChatFeature.State(server: mini, target: .thread("r1"), persona: Persona(coder))
        room.room = Room(title: "R", members: ["coder"], cast: [coder])
        let rooms = TestStore(initialState: room) { ChatFeature() }
        await rooms.send(.machine(.load))
    }

    @Test internal func aChatRunsOnItsThreadsMachineAndANewOneKeepsItsPickOnceStarted() async {
        var state = AppFeature.State(layout: testLayout, server: mini)
        let onAir = APIPersona(id: "coder", name: "Coder", agent: "codex", machine: "box")
        state.home?.personas = [onAir]
        state.home?.rows = [
            ThreadRow(
                id: "t2", title: "T", agent: "codex", persona: "coder", state: "done", last: "", updated: 1, unseen: 0,
                archived: false, machine: "mini")
        ]
        let store = TestStore(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.threads.home = { _ in HomeAnswer(screen: homeScreen, threads: [], personas: [onAir]) }
            $0.threads.personas = { _ in [onAir] }
            $0.plugins.installed = { _ in [] }
        }
        store.exhaustivity = .off(showSkippedAssertions: false)
        await store.send(.open("t2"))
        #expect(store.state.chat?.machine.current == "mini")
        await store.send(.start("coder"))
        #expect(store.state.chat?.machine.current == "box")
        await store.send(.chat(.machine(.pick(""))))
        await store.send(.chat(.delegate(.started("n1"))))
        #expect(store.state.chat?.target == .thread("n1") && store.state.chat?.machine.current == "")
        await store.skipReceivedActions()
    }
}
