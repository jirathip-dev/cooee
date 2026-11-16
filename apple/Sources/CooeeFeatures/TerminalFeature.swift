import ComposableArchitecture
import CooeeAPI
import CooeeCore
import Foundation

/// What goes into a shell, in the order it was typed: text, or named keys (`enter`, `ctrl+c`).
public enum TerminalInput: Sendable, Equatable {
    case text(Keystrokes)
    case keys([String])
}

@DependencyClient
public struct TerminalClient: Sendable {
    public var list: @Sendable (_ server: Server) async throws -> [TerminalShell]
    public var shell: @Sendable (_ server: Server) async throws -> String
    public var input: @Sendable (_ server: Server, _ terminal: String, _ input: TerminalInput) async throws -> Void
    public var resize: @Sendable (_ server: Server, _ terminal: String, _ size: TerminalSize) async throws -> Void
    public var watch:
        @Sendable (_ server: Server, _ terminal: String) -> AsyncThrowingStream<TerminalUpdate, any Error> = { _, _ in
            .finished()
        }
}

@Reducer
public struct TerminalFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public var server: Server?
        public var size: TerminalSize?
        public var shells: [TerminalShell] = []
        public var current: TerminalShell?
        public var screen = TerminalScreen()
        public var ctrl = false
        public var note: String?
        internal var sending = false
        internal var pending: [TerminalInput] = []

        public init(server: Server?) {
            self.server = server
        }
    }

    public enum Action: Sendable {
        case task(TerminalSize?)
        case close
        case listed([TerminalShell], open: String?)
        case open(String)
        case update(TerminalUpdate)
        case newShell
        case type(Keystrokes)
        case press(String)
        case ctrl
        case sent
        case failed(String)
    }

    private enum CancelID { case watch }

    @Dependency(\.terminal) private var client

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .task(let size):
                state.size = size
                return list(state)
            case .close: return .cancel(id: CancelID.watch)
            case .listed(let shells, let name):
                state.shells = shells
                guard let pick = shells.first(where: { $0.name == name ?? state.current?.name }) ?? shells.first else {
                    state.current = nil
                    state.note = "No shells yet: New shell starts one."
                    return .none
                }
                return open(&state, pick)
            case .open(let name): return state.shells.first { $0.name == name }.map { open(&state, $0) } ?? .none
            case .update(let update): state.screen.apply(update)
            case .newShell: return list(state, start: true)
            case .type(let keystrokes): return type(&state, keystrokes)
            case .press(let key): return input(&state, .keys([key]))
            case .ctrl: state.ctrl.toggle()
            case .sent: return sent(&state)
            case .failed(let why): state.note = why
            }
            return .none
        }
    }

    private static func why(_ error: any Error) -> String {
        guard (error as? APIError)?.status == 403 else { return error.localizedDescription }
        return "Locked: unlock this device with its passkey first. Your shells keep running."
    }

    private func list(_ state: State, start: Bool = false) -> Effect<Action> {
        guard let server = state.server else { return .none }
        return .run { send in
            let name = start ? try await client.shell(server) : nil
            await send(.listed(try await client.list(server), open: name))
        } catch: { error, send in
            await send(.failed(Self.why(error)))
        }
    }

    private func open(_ state: inout State, _ shell: TerminalShell) -> Effect<Action> {
        guard let server = state.server else { return .none }
        state.current = shell
        state.screen = TerminalScreen()
        state.pending = []
        state.note = shell.owner ? nil : "\(shell.name) is an agent's terminal: read-only."
        let size = shell.owner ? state.size : nil
        return .run { send in
            if let size { try await client.resize(server, shell.name, size) }
            for try await update in client.watch(server, shell.name) { await send(.update(update)) }
            let left = try await client.list(server).filter { $0.name != shell.name }
            await send(.listed(left, open: nil))
        } catch: { error, send in
            await send(.failed(Self.why(error)))
        }
        .cancellable(id: CancelID.watch, cancelInFlight: true)
    }

    private func type(_ state: inout State, _ keystrokes: Keystrokes) -> Effect<Action> {
        let letter = keystrokes.text.count == 1 && keystrokes.text.allSatisfy(\.isLetter)
        guard state.ctrl, letter else { return input(&state, .text(keystrokes)) }
        state.ctrl = false
        return input(&state, .keys(["ctrl+\(keystrokes.text.lowercased())"]))
    }

    private func input(_ state: inout State, _ input: TerminalInput) -> Effect<Action> {
        guard let server = state.server, let shell = state.current, shell.owner else { return .none }
        guard !state.sending else {
            state.pending.append(input)
            return .none
        }
        state.sending = true
        return .run { send in
            try await client.input(server, shell.name, input)
            await send(.sent)
        } catch: { error, send in
            await send(.failed(Self.why(error)))
            await send(.sent)
        }
    }

    private func sent(_ state: inout State) -> Effect<Action> {
        state.sending = false
        guard !state.pending.isEmpty else { return .none }
        return input(&state, state.pending.removeFirst())
    }
}

extension TerminalClient: TestDependencyKey {
    public static let testValue = TerminalClient()
}

extension DependencyValues {
    public var terminal: TerminalClient {
        get { self[TerminalClient.self] }
        set { self[TerminalClient.self] = newValue }
    }
}
