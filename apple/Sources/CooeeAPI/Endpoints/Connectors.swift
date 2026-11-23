import CooeeCore
import Foundation

extension Client {
    public func connectors() async throws -> ConnectorList { try await call("GET", "/connectors") }

    public func addConnector(_ name: String) async throws { try await post("/connectors", ["catalogue": name]) }

    public func removeConnector(_ name: String) async throws {
        _ = try await send(request("DELETE", "/connectors/\(name)"))
    }

    /// Starts a sign-in whose callback comes back to this client's own origin (the Mac's tailnet name).
    public func signIn(_ connector: String, account: String) async throws -> SignInStart {
        let port = base.port.map { ":\($0)" } ?? ""
        let origin = "\(base.scheme ?? "https")://\(base.host() ?? "")\(port)"
        return try await call("POST", "/connectors/\(connector)/signin", ["account": account, "origin": origin])
    }

    public func signInFlow(_ id: String) async throws -> SignInFlow { try await call("GET", "/connectors/flows/\(id)") }

    public func paste(_ connector: String, flow: String, token: String) async throws {
        try await post("/connectors/\(connector)/token", ["flow": flow, "value": token])
    }

    public func signOut(_ connector: String, account: String) async throws {
        try await post("/connectors/\(connector)/signout", ["account": account])
    }
}
