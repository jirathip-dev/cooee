import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension PairingClient: DependencyKey {
    public static let liveValue = PairingClient(
        pair: { url, code in try await Client(base: url).pair(code: code) },
        ask: { url in try await Client(base: url).askToPair() },
        claim: { url, claim in try await Client(base: url).claim(claim) },
        cancel: { url, claim in try await Client(base: url).cancelPair(claim) }
    )
}
