import CooeeCore

extension Client {
    public func create(_ persona: APIPersona) async throws -> APIPersona {
        try await call("POST", "/personas", persona)
    }

    public func save(_ persona: APIPersona) async throws -> APIPersona {
        try await call("PUT", "/personas/\(persona.id)", persona)
    }

    public func memory(_ persona: String) async throws -> [PersonaNote] {
        try await call("GET", "/personas/\(persona)/memory")
    }

    public func forget(_ persona: String) async throws {
        _ = try await send(request("DELETE", "/personas/\(persona)/memory"))
    }

    public func push(_ token: String, enabled: Bool) async throws {
        _ = try await send(request(enabled ? "POST" : "DELETE", "/push/apns", json: ["token": token]))
    }
}
