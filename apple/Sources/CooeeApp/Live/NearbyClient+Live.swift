import ComposableArchitecture
import CooeeAPI
import CooeeFeatures
import Foundation

extension NearbyClient: DependencyKey {
    #if os(macOS)
        public static let liveValue = NearbyClient {
            Nearby.browse(local: URL(string: "http://127.0.0.1:\(LocalConfig.standard.port)"))
        }
    #else
        public static let liveValue = NearbyClient { Nearby.browse() }
    #endif
}
