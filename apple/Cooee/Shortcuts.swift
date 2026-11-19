import AppIntents
import CooeeApp
import CooeeFeatures

internal struct CooeeIntents: AppIntentsPackage {
    internal static var includedPackages: [any AppIntentsPackage.Type] { [SurfaceIntents.self] }
}

internal struct CooeeShortcuts: AppShortcutsProvider {
    internal static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: OpenNeedsYouIntent(), phrases: ["What needs me in \(.applicationName)"],
            shortTitle: "What needs me", systemImageName: "exclamationmark.bubble.fill")
        AppShortcut(
            intent: StartChatIntent(), phrases: ["New chat with \(\.$persona) in \(.applicationName)"],
            shortTitle: "New chat", systemImageName: "plus.bubble.fill")
        AppShortcut(
            intent: SendToThreadIntent(), phrases: ["Tell \(\.$persona) in \(.applicationName)"],
            shortTitle: "Tell a persona", systemImageName: "waveform")
        AppShortcut(
            intent: StopTurnIntent(), phrases: ["Stop the running turn in \(.applicationName)"],
            shortTitle: "Stop the running turn", systemImageName: "stop.fill")
        AppShortcut(
            intent: OpenAskIntent(), phrases: ["Open \(\.$thread) in \(.applicationName)"],
            shortTitle: "Open a thread", systemImageName: "text.bubble.fill")
    }
}
