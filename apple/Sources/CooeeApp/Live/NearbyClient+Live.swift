import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension NearbyClient: DependencyKey {
    public static let liveValue = NearbyClient(browse: { Nearby.browse() })
}
