import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension ServerStore: DependencyKey {
    public static let liveValue = ServerStore(
        load: { Keychain().load() },
        save: { server in Keychain().save(server) },
        clear: { Keychain().clear() }
    )
}
