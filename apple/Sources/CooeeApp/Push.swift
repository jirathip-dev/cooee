import CooeeAPI
import CooeeFeatures
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
    private static var denied: AsyncStream<PushSaid>.Continuation?
    private static var heldDenied: [PushSaid] = []

    override public init() {
        super.init()
        let center = UNUserNotificationCenter.current()
        center.delegate = self
        let say = UNTextInputNotificationAction(
            identifier: "say", title: "Deny and say…", textInputButtonTitle: "Send",
            textInputPlaceholder: "What to do instead")
        center.setNotificationCategories([.init(identifier: "approval", actions: [say], intentIdentifiers: [])])
    }

    nonisolated public func userNotificationCenter(
        _ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse
    ) async {
        let content = response.notification.request.content
        guard !content.threadIdentifier.isEmpty else { return }
        if let reply = response as? UNTextInputNotificationResponse, let item = content.userInfo["item"] as? Int {
            return await Self.deny(PushSaid(thread: content.threadIdentifier, item: item, text: reply.userText))
        }
        await Self.open(content.threadIdentifier)
    }

    internal static func said() -> AsyncStream<PushSaid> {
        let (stream, continuation) = AsyncStream.makeStream(of: PushSaid.self)
        denied = continuation
        for said in heldDenied { continuation.yield(said) }
        heldDenied = []
        return stream
    }

    private static func deny(_ said: PushSaid) {
        if let denied { denied.yield(said) } else { heldDenied.append(said) }
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
