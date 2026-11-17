import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension ScreenCache: DependencyKey {
    public static let liveValue = ScreenCache(
        load: { key in KeptScreens().load(key) },
        save: { key, kept in KeptScreens().save(key, kept) }
    )
}
