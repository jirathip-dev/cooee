import CooeeCore
import Foundation

extension Client {
    private struct Named: Decodable {
        fileprivate let terminal: String
    }

    private struct Typed: Encodable {
        fileprivate let terminal: String
        fileprivate let text: String
    }

    private struct Keys: Encodable {
        fileprivate let terminal: String
        fileprivate let keys: [String]
    }

    private struct Size: Encodable {
        fileprivate let terminal: String
        fileprivate let cols: Int
        fileprivate let rows: Int
    }

    private static let terminal = "/p/terminal/api"

    public func terminals() async throws -> [TerminalShell] {
        try await call("GET", "\(Self.terminal)/list")
    }

    public func newShell() async throws -> String {
        let started: Named = try await call("POST", "\(Self.terminal)/shell", [String: String]())
        return started.terminal
    }

    public func type(_ keystrokes: Keystrokes, into terminal: String) async throws {
        try await post("\(Self.terminal)/input", Typed(terminal: terminal, text: keystrokes.text))
    }

    public func press(_ keys: [String], in terminal: String) async throws {
        try await post("\(Self.terminal)/keys", Keys(terminal: terminal, keys: keys))
    }

    public func resize(_ terminal: String, to size: TerminalSize) async throws {
        try await post("\(Self.terminal)/resize", Size(terminal: terminal, cols: size.cols, rows: size.rows))
    }

    /// The terminal's screen updates; the stream ends when its program exits.
    public func watch(_ terminal: String) -> AsyncThrowingStream<TerminalUpdate, any Error> {
        let events = screenEvents(.plugin("terminal"), "/api/watch?t=\(terminal)")
        return AsyncThrowingStream { continuation in
            let task = Task {
                do {
                    for try await event in events {
                        guard event.string("event") == "screen" else { break }
                        let data = Data((event.string("data") ?? "").utf8)
                        continuation.yield(try JSONDecoder().decode(TerminalUpdate.self, from: data))
                    }
                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }
}
