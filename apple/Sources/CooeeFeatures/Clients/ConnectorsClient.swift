import ComposableArchitecture
import CooeeCore
import Foundation

@DependencyClient
public struct ConnectorsClient: Sendable {
    public var list: @Sendable (_ server: Server) async throws -> ConnectorList
    public var add: @Sendable (_ server: Server, _ name: String) async throws -> Void
    public var remove: @Sendable (_ server: Server, _ name: String) async throws -> Void
    public var signIn: @Sendable (_ server: Server, _ name: String, _ account: String) async throws -> SignInStart
    public var flow: @Sendable (_ server: Server, _ id: String) async throws -> SignInFlow
    public var paste: @Sendable (_ server: Server, _ name: String, _ flow: String, _ token: String) async throws -> Void
    public var signOut: @Sendable (_ server: Server, _ name: String, _ account: String) async throws -> Void
}

/// The system's web sign-in sheet: opens `url` and returns once it reaches a URL of `scheme` (cooee's callback).
@DependencyClient
public struct WebSignInClient: Sendable {
    public var open: @Sendable (_ url: URL, _ scheme: String) async throws -> Void
}

extension ConnectorsClient: TestDependencyKey {
    public static let testValue = ConnectorsClient()
}

extension WebSignInClient: TestDependencyKey {
    public static let testValue = WebSignInClient()
}

extension DependencyValues {
    public var connectors: ConnectorsClient {
        get { self[ConnectorsClient.self] }
        set { self[ConnectorsClient.self] = newValue }
    }

    public var webSignIn: WebSignInClient {
        get { self[WebSignInClient.self] }
        set { self[WebSignInClient.self] = newValue }
    }
}
