import CooeeAPI
import SwiftUI
import UserNotifications

#if os(iOS)
    import UIKit

    public typealias Platform = UIApplication
    public typealias PlatformDelegate = UIApplicationDelegate
#else
    import AppKit

    public typealias Platform = NSApplication
    public typealias PlatformDelegate = NSApplicationDelegate
#endif

@MainActor
public final class PushDelegate: NSObject, PlatformDelegate {
    private static var waiting: CheckedContinuation<String, any Error>?

    public func application(_ application: Platform, didRegisterForRemoteNotificationsWithDeviceToken token: Data) {
        Self.waiting?.resume(returning: token.map { String(format: "%02x", $0) }.joined())
        Self.waiting = nil
    }

    public func application(_ application: Platform, didFailToRegisterForRemoteNotificationsWithError error: any Error)
    {
        Self.waiting?.resume(throwing: error)
        Self.waiting = nil
    }

    nonisolated internal static func token() async throws -> String {
        let center = UNUserNotificationCenter.current()
        guard try await center.requestAuthorization(options: [.alert, .badge, .sound]) else {
            throw APIError(status: 0, text: "Notifications are off for cooee in Settings.")
        }
        return try await register()
    }

    private static func register() async throws -> String {
        try await withCheckedThrowingContinuation { continuation in
            waiting?.resume(throwing: CancellationError())
            waiting = continuation
            Platform.shared.registerForRemoteNotifications()
        }
    }
}
