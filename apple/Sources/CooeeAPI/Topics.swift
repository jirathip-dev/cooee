import CooeeCore
import Foundation

extension Client {
    private struct Archive: Encodable {
        private enum CodingKeys: String, CodingKey {
            case archived = "on"
        }

        fileprivate let archived: Bool
    }

    public func topics(of persona: String) async throws -> [ThreadRow] {
        let id = persona.addingPercentEncoding(withAllowedCharacters: .alphanumerics) ?? persona
        return try await call("GET", "/threads?persona=\(id)")
    }

    public func retitle(_ id: String, _ title: String) async throws {
        try await post("/threads/\(id)/title", ["title": title])
    }

    public func archive(_ id: String, _ archived: Bool) async throws {
        try await post("/threads/\(id)/archive", Archive(archived: archived))
    }
}
