import ComposableArchitecture
import CooeeCore

@DependencyClient
public struct PersonasClient: Sendable {
    public var create: @Sendable (_ server: Server, _ persona: APIPersona) async throws -> APIPersona
    public var remove: @Sendable (_ server: Server, _ persona: String) async throws -> Void
    public var agents: @Sendable (_ server: Server) async throws -> [AgentOption]
    public var models: @Sendable (_ server: Server) async throws -> [ModelList]
    public var machines: @Sendable (_ server: Server) async throws -> [MachineRow]
    public var folders: @Sendable (_ server: Server, _ path: String?) async throws -> FolderListing
    public var rules: @Sendable (_ server: Server, _ persona: String) async throws -> [Rule]
    public var addRule: @Sendable (_ server: Server, _ persona: String, _ rule: NewRule) async throws -> Rule
    public var removeRule: @Sendable (_ server: Server, _ persona: String, _ rule: String) async throws -> Void
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
