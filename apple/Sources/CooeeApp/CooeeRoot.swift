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
    private static let live = Store(
        initialState: AppFeature.State(
            layout: (try? LayoutStore.liveValue.load()) ?? .bundled, server: ServerStore.liveValue.load(),
            settings: SettingsStore.liveValue.load()
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
