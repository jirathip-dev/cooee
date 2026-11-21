import ComposableArchitecture
import CooeeAPI
import CooeeFeatures
import Foundation

#if os(iOS)
    import UIKit

    private let leftFront = UIApplication.didEnterBackgroundNotification
#else
    import AppKit

    private let leftFront = NSApplication.didResignActiveNotification
#endif

extension ScreenCache: DependencyKey {
    public static let liveValue = ScreenCache(
        load: { key in KeptScreens().load(key) },
        save: { key, kept in KeptScreens().save(key, kept) },
        backgrounds: {
            AsyncStream { continuation in
                let watch = Task {
                    for await _ in NotificationCenter.default.notifications(named: leftFront) { continuation.yield() }
                    continuation.finish()
                }
                continuation.onTermination = { _ in watch.cancel() }
            }
        }
    )
}
