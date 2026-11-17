#if os(macOS)
    import ComposableArchitecture
    import CooeeAPI
    import CooeeFeatures
    import CooeeUI
    import Foundation
    import Sparkle
    import SwiftUI

    @MainActor
    internal final class Updater: NSObject, SPUUpdaterDelegate {
        internal static let shared = Updater()

        private var controller: SPUStandardUpdaterController?

        internal var status: UpdateStatus {
            guard let updater = controller?.updater else { return .unsigned }
            return .signed(automatic: updater.automaticallyDownloadsUpdates)
        }

        override private init() {
            super.init()
            let key = Bundle.main.object(forInfoDictionaryKey: "SUPublicEDKey") as? String ?? ""
            guard !key.isEmpty else { return }
            controller = SPUStandardUpdaterController(
                startingUpdater: true, updaterDelegate: self, userDriverDelegate: nil)
        }

        internal func check() {
            controller?.updater.checkForUpdates()
        }

        internal func setAutomatic(_ enabled: Bool) {
            controller?.updater.automaticallyChecksForUpdates = enabled
            controller?.updater.automaticallyDownloadsUpdates = enabled
        }

        nonisolated internal func updater(
            _ updater: SPUUpdater, willInstallUpdateOnQuit item: SUAppcastItem,
            immediateInstallationBlock immediateInstallHandler: @escaping () -> Void
        ) -> Bool {
            LoginItem.restartAfterUpdate(port: LocalConfig.standard.port)
            return false
        }

        nonisolated internal func updater(_ updater: SPUUpdater, willInstallUpdate item: SUAppcastItem) {
            LoginItem.restartAfterUpdate(port: LocalConfig.standard.port)
        }
    }

    extension UpdaterClient: DependencyKey {
        public static let liveValue = UpdaterClient(
            check: { await Updater.shared.check() },
            setAutomatic: { await Updater.shared.setAutomatic($0) }
        )
    }

    @MainActor
    public enum Updates {
        private static let store = Store(initialState: UpdatesFeature.State(status: Updater.shared.status)) {
            UpdatesFeature()
        }

        public static var commands: some Commands { UpdateCommands(store: store) }

        public static var settings: some View { UpdateSettings(store: store) }
    }
#endif
