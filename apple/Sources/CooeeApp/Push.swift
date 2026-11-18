import CooeeAPI
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
public final class PushDelegate: NSObject, PlatformDelegate, UNUserNotificationCenterDelegate {
    private static var waiting: CheckedContinuation<String, any Error>?
    private static var tapped: AsyncStream<String>.Continuation?
    private static var held: [String] = []

    override public init() {
        super.init()
        UNUserNotificationCenter.current().delegate = self
    }

    nonisolated public func userNotificationCenter(
        _ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse
    ) async {
        let thread = response.notification.request.content.threadIdentifier
        guard !thread.isEmpty else { return }
        await Self.open(thread)
    }

    internal static func taps() -> AsyncStream<String> {
        let (stream, continuation) = AsyncStream.makeStream(of: String.self)
        tapped = continuation
        for thread in held { continuation.yield(thread) }
        held = []
        return stream
    }

    private static func open(_ thread: String) {
        if let tapped { tapped.yield(thread) } else { held.append(thread) }
    }

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
