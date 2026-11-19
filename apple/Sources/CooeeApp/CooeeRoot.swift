import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import CooeeUI
import SwiftUI

public struct CooeeRoot: View {
    #if os(macOS)
        private static let mac = true
    #else
        private static let mac = false
    #endif
    internal static let live = Store(
        initialState: AppFeature.State.launching(
            layout: (try? LayoutStore.liveValue.load()) ?? .bundled, settings: SettingsStore.liveValue.load()
        )
    ) { AppFeature() }
    private let store = Self.live

    public var body: some View {
        AppRoot(store: store).task {
            if Self.mac { try? await AppIconClient.liveValue.set(SettingsStore.liveValue.load().appIcon) }
        }
    }

    public init() {}
}
