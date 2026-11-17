import ComposableArchitecture
import CooeeCore

@DependencyClient
public struct SettingsStore: Sendable {
    public var load: @Sendable () -> Settings = { Settings() }
    public var save: @Sendable (_ settings: Settings) -> Void
}

extension SettingsStore: TestDependencyKey {
    public static let testValue = SettingsStore()
}

extension DependencyValues {
    public var settingsStore: SettingsStore {
        get { self[SettingsStore.self] }
        set { self[SettingsStore.self] = newValue }
    }
}
