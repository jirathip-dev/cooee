import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension TerminalClient: DependencyKey {
    public static let liveValue = TerminalClient(
        list: { server in try await Client(server).terminals() },
        shell: { server in try await Client(server).newShell() },
        input: { server, terminal, input in
            switch input {
            case .text(let keystrokes): try await Client(server).type(keystrokes, into: terminal)
            case .keys(let keys): try await Client(server).press(keys, in: terminal)
            }
        },
        resize: { server, terminal, size in try await Client(server).resize(terminal, to: size) },
        watch: { server, terminal in Client(server).watch(terminal) }
    )
}
