import ComposableArchitecture
import CooeeCore

@DependencyClient
public struct PersonasClient: Sendable {
    public var create: @Sendable (_ server: Server, _ persona: APIPersona) async throws -> APIPersona
    public var save: @Sendable (_ server: Server, _ persona: APIPersona) async throws -> APIPersona
    public var memory: @Sendable (_ server: Server, _ persona: String) async throws -> [PersonaNote]
    public var forget: @Sendable (_ server: Server, _ persona: String) async throws -> Void
    public var agents: @Sendable (_ server: Server) async throws -> [AgentOption]
    public var models: @Sendable (_ server: Server) async throws -> [ModelList]
    public var folders: @Sendable (_ server: Server, _ path: String?) async throws -> FolderListing
}

extension PersonasClient: TestDependencyKey {
    public static let testValue = PersonasClient()
}

extension DependencyValues {
    public var personas: PersonasClient {
        get { self[PersonasClient.self] }
        set { self[PersonasClient.self] = newValue }
    }
}
