import CooeeCore
import Foundation

extension Client {
    private struct Gone: Decodable {
        fileprivate let why: String
    }

    public func screen(_ persona: String) async throws -> BrowserScreen {
        try await call("GET", "/personas/\(persona)/screen")
    }

    public func takeControl(_ persona: String) async throws -> BrowserScreen {
        try await call("POST", "/personas/\(persona)/screen/control", [String: String]())
    }

    public func handBack(_ persona: String, note: String) async throws -> BrowserScreen {
        try await call("POST", "/personas/\(persona)/screen/back", ["note": note])
    }

    public func input(_ persona: String, _ input: BrowserInput) async throws {
        try await post("/personas/\(persona)/screen/input", input)
    }

    public func live(_ persona: String) -> AsyncThrowingStream<LiveEvent, any Error> {
        AsyncThrowingStream { continuation in
            let task = Task {
                while !Task.isCancelled {
                    do {
                        _ = try await sse(request("GET", "/personas/\(persona)/screen/live"), id: "") { frame in
                            if let event = try Self.live(frame) { continuation.yield(event) }
                            return false
                        }
                    } catch let error as APIError where error.status < 500 {
                        continuation.finish(throwing: error)
                        return
                    } catch {
                        try? await clock.sleep(for: retry)
                        continue
                    }
                    try? await clock.sleep(for: retry)
                }
                continuation.finish()
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }

    internal static func live(_ frame: SSEFrame) throws -> LiveEvent? {
        let data = Data(frame.data.utf8)
        switch frame.event {
        case "frame": return .frame(try JSONDecoder().decode(LiveFrame.self, from: data))
        case "gone": return .gone(try JSONDecoder().decode(Gone.self, from: data).why)
        default: return nil
        }
    }
}
