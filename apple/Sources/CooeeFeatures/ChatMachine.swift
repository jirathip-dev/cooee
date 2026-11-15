import ComposableArchitecture
import CooeeCore

public struct MachineChip: Equatable, Sendable {
    public var current: String
    public var machines: [MachineRow] = []
    public var open = false
    public var moving = false
    public var refusal: String?

    public var shown: MachineRow? {
        guard !machines.isEmpty else { return nil }
        return machines.first { $0.name == current } ?? MachineRow(name: current, host: current, online: false)
    }

    public init(current: String = "") {
        self.current = current
    }
}

extension ChatFeature.Target {
    public var started: Bool {
        guard case .thread = self else { return false }
        return true
    }
}

extension ChatFeature.Action {
    @CasePathable
    public enum MachineAction: Sendable, Equatable {
        case load
        case loaded([MachineRow])
        case show(Bool)
        case pick(String)
        case moved(String)
        case refused(String)
    }
}

extension ChatFeature {
    internal func machine(_ state: inout State, _ action: Action.MachineAction) -> Effect<Action> {
        switch action {
        case .load:
            return state.room == nil ? machines(state) : .none
        case .loaded(let machines):
            state.machine.machines = machines
        case .show(let open):
            (state.machine.open, state.machine.refusal) = (open, nil)
        case .pick(let name):
            return pick(&state, name)
        case .moved(let name):
            (state.machine.current, state.machine.moving, state.machine.open) = (name, false, false)
        case .refused(let why):
            (state.machine.moving, state.machine.refusal) = (false, why)
        }
        return .none
    }

    private func pick(_ state: inout State, _ name: String) -> Effect<Action> {
        guard name != state.machine.current, !state.machine.moving else { return .none }
        guard case .thread(let thread) = state.target else {
            (state.machine.current, state.machine.open) = (name, false)
            return .none
        }
        (state.machine.moving, state.machine.refusal) = (true, nil)
        let (threads, server) = (threads, state.server)
        return .run { send in
            try await threads.move(server, thread, name)
            await send(.machine(.moved(name)))
        } catch: { error, send in
            await send(.machine(.refused(error.localizedDescription)))
        }
    }

    private func machines(_ state: State) -> Effect<Action> {
        let (personas, server) = (personas, state.server)
        return .run { send in
            await send(.machine(.loaded(try await personas.machines(server))))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}
