import ComposableArchitecture
import CooeeCore
import CooeeFeatures

#if os(iOS)
    import UIKit
#else
    import AppKit
#endif

extension AppIconClient: DependencyKey {
    public static let liveValue = AppIconClient(set: { icon in try await apply(icon) })

    @MainActor private static func apply(_ icon: AppIcon) async throws {
        #if os(iOS)
            guard UIApplication.shared.alternateIconName != icon.alternateName else { return }
            try await UIApplication.shared.setAlternateIconName(icon.alternateName)
        #else
            NSApplication.shared.applicationIconImage = NSImage(named: "\(icon.asset)Preview")
        #endif
    }
}
