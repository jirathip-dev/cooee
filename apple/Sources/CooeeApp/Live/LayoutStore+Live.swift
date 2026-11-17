import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension LayoutStore: DependencyKey {
    public static let liveValue = LayoutStore.onDisk(LayoutFile(), bundled: .bundled)
}
