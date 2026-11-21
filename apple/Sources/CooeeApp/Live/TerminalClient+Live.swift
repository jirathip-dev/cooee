import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension TerminalClient: DependencyKey {
    public static let liveValue = TerminalClient(
        list: { server in try await client(server).terminals() },
        shell: { server in try await client(server).newShell() },
        input: { server, terminal, input in
            switch input {
            case .text(let keystrokes): try await client(server).type(keystrokes, into: terminal)
            case .keys(let keys): try await client(server).press(keys, in: terminal)
            }
        },
        resize: { server, terminal, size in try await client(server).resize(terminal, to: size) },
        watch: { server, terminal in client(server).watch(terminal) }
    )
}
