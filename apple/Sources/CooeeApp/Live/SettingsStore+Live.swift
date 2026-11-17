import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import Foundation

extension SettingsStore: DependencyKey {
    private static let key = "cooee.settings"

    public static let liveValue = SettingsStore(
        load: {
            let data = UserDefaults.standard.data(forKey: key) ?? Data()
            return (try? JSONDecoder().decode(Settings.self, from: data)) ?? Settings()
        },
        save: { settings in UserDefaults.standard.set(try? JSONEncoder().encode(settings), forKey: key) }
    )
}
