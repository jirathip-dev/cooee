import CooeeCore
import Foundation

extension Client {
    public func create(_ persona: APIPersona) async throws -> APIPersona {
        try await call("POST", "/personas", persona)
    }

    public func remove(_ persona: String) async throws {
        _ = try await send(request("DELETE", "/personas/\(persona)"))
    }

    public func model(_ thread: String, model: String, effort: String) async throws {
        try await post("/threads/\(thread)/model", ["model": model, "effort": effort])
    }

    public func switchAgent(_ thread: String, to agent: String) async throws {
        try await post("/threads/\(thread)/agent", ["agent": agent])
    }

    public func push(_ token: String, enabled: Bool) async throws {
        _ = try await send(request(enabled ? "POST" : "DELETE", "/push/apns", json: ["token": token]))
    }

    /// This device's push subscriptions on the server: its APNs tokens, lowercased (#373).
    public func pushed() async throws -> [String] { try await call("GET", "/push/endpoints") }

    /// A test alert to this device's subscriptions (#373).
    public func pushTest() async throws { _ = try await send(request("POST", "/push/test")) }

    /// What the widgets show: the asks, the blocked lanes and what runs (#662).
    public func glance() async throws -> Snapshot { try await call("GET", "/glance") }

    /// The Live Activity's push-to-start token (no `activity`), or the running activity's update token (#662).
    public func activity(_ token: String, for activity: String?) async throws {
        var body = ["token": token]
        body["activity"] = activity
        try await post("/push/activity", body)
    }

    /// One of the app's timings into the server's perf log (#606).
    public func perf(_ name: String, millis: Int) async throws {
        try await post("/perf", Sample(name: name, millis: millis))
    }

    /// A report-mode note into the server's reports (#376), listed by `cooee reports`.
    public func report(_ report: Report) async throws {
        try await post("/report", report)
    }
}

private struct Sample: Encodable {
    private enum CodingKeys: String, CodingKey {
        case name
        case millis = "ms"
    }

    fileprivate let name: String
    fileprivate let millis: Int
}

extension Client {
    public func agents() async throws -> [AgentOption] { try await call("GET", "/agents") }

    public func models() async throws -> [ModelList] { try await call("GET", "/models") }

    public func machines() async throws -> [MachineRow] { try await call("GET", "/machines") }

    public func rules(_ persona: String) async throws -> [Rule] { try await call("GET", "/personas/\(persona)/rules") }

    public func addRule(_ persona: String, _ rule: NewRule) async throws -> Rule {
        try await call("POST", "/personas/\(persona)/rules", rule)
    }

    public func removeRule(_ persona: String, _ rule: String) async throws {
        _ = try await send(request("DELETE", "/personas/\(persona)/rules/\(rule)"))
    }

    public func folders(_ path: String?) async throws -> FolderListing {
        let query = path.map { "?path=" + ($0.addingPercentEncoding(withAllowedCharacters: .alphanumerics) ?? "") }
        return try await call("GET", "/folders" + (query ?? ""))
    }
}
