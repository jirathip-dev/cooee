import CooeeCore

extension Client {
    public func home() async throws -> HomeAnswer { try await call("GET", "/screens/home") }

    public func personas() async throws -> [APIPersona] { try await call("GET", "/personas") }
}
