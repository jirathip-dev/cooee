import CooeeCore
import Foundation

extension Client {
    private struct Archive: Encodable {
        private enum CodingKeys: String, CodingKey {
            case archived = "on"
        }

        fileprivate let archived: Bool
    }

    public func topics(of owner: TopicsOf) async throws -> [ThreadRow] {
        let (key, id) =
            switch owner {
            case .persona(let id): ("persona", id)
            case .room(let id): ("room", id)
            }
        let escaped = id.addingPercentEncoding(withAllowedCharacters: .alphanumerics) ?? id
        return try await call("GET", "/threads?\(key)=\(escaped)")
    }

    public func retitle(_ id: String, _ title: String) async throws {
        try await post("/threads/\(id)/title", ["title": title])
    }

    public func archive(_ id: String, _ archived: Bool) async throws {
        try await post("/threads/\(id)/archive", Archive(archived: archived))
    }

    public func close(_ id: String) async throws {
        try await post("/threads/\(id)/close", [String: String]())
    }
}
