import ComposableArchitecture
import CooeeCore
import Foundation

@DependencyClient
public struct PairingClient: Sendable {
    public var pair: @Sendable (_ url: URL, _ code: String) async throws -> String
    public var ask: @Sendable (_ url: URL) async throws -> PairAsk
    public var claim: @Sendable (_ url: URL, _ claim: String) async throws -> PairClaim
    public var cancel: @Sendable (_ url: URL, _ claim: String) async throws -> Void
}

extension PairingClient: TestDependencyKey {
    public static let testValue = PairingClient()
}

extension DependencyValues {
    public var pairing: PairingClient {
        get { self[PairingClient.self] }
        set { self[PairingClient.self] = newValue }
    }
}
