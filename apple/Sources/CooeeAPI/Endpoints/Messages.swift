import CooeeCore
import Foundation

extension Client {
    public func messages(_ id: String, before: Int = 50, around: Int? = nil) async throws -> Slice {
        try await call("GET", "/threads/\(id)/messages?before=\(before)" + (around.map { "&around=\($0)" } ?? ""))
    }

    public func frame(_ id: String, _ name: String) async throws -> Data {
        try await send(request("GET", "/threads/\(id)/frames/\(name)")).0
    }

    public func file(_ id: String, _ name: String) async throws -> Data {
        try await send(request("GET", "/threads/\(id)/files/\(name)")).0
    }

    public func read(_ id: String, upTo item: Int) async throws {
        try await post("/threads/\(id)/read", ["item": item])
    }
}
