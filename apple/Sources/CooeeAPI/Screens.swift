import CooeeCore
import Foundation

extension Client {
    private struct Installed: Decodable {
        fileprivate let installed: [PluginRow]
    }

    private struct ScreenAction: Encodable {
        fileprivate let screen: String
        fileprivate let block: String
        fileprivate let action: String
        fileprivate let values: [String: JSONValue]
    }

    public func places(_ persona: String) async throws -> [Place] {
        try await call("GET", "/personas/\(persona)/places")
    }

    public func plugins() async throws -> [PluginRow] {
        let list: Installed = try await call("GET", "/plugins")
        return list.installed
    }

    private static func prefix(_ source: LayoutSource) -> String {
        switch source {
        case .core: ""
        case .plugin(let plugin): "/p/\(plugin)"
        }
    }

    public func screen(_ source: LayoutSource, _ path: String) async throws -> JSONValue {
        try await call("GET", Self.prefix(source) + path)
    }

    public func act(_ source: LayoutSource, screen: String, _ action: BlockAction) async throws -> JSONValue {
        let body = ScreenAction(screen: screen, block: "", action: action.action, values: action.values)
        let path = source == .core ? "/screens/action" : "/p/\(source.name)/action"
        return try await call("POST", path, body)
    }

    public func attach(_ plugin: String, to persona: String) async throws {
        let found: JSONValue = try await call("GET", "/personas/\(persona)")
        var fields = found.fields
        let plugins = fields["plugins"]?.items ?? []
        guard !plugins.contains(.string(plugin)) else { return }
        fields["plugins"] = .array(plugins + [.string(plugin)])
        _ = try await send(request("PUT", "/personas/\(persona)", json: JSONValue.object(fields)))
    }

    public func screenEvents(_ source: LayoutSource, _ path: String) -> AsyncThrowingStream<JSONValue, any Error> {
        AsyncThrowingStream { continuation in
            let task = Task {
                do {
                    let (bytes, response) = try await session.bytes(for: request("GET", Self.prefix(source) + path))
                    let status = try Self.http(response).statusCode
                    if status >= 400 { throw APIError(status: status, text: "") }
                    for try await line in bytes.lines where line.hasPrefix("data:") {
                        continuation.yield(try JSONDecoder().decode(JSONValue.self, from: Data(line.dropFirst(5).utf8)))
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
