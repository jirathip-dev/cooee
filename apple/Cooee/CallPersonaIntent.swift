import AppIntents
import CooeeApp

internal struct CallPersonaIntent: AppIntent {
    internal static let title: LocalizedStringResource = "Call a persona"
    internal static let description = IntentDescription("Start a voice call with a persona in its latest thread.")
    internal static let openAppWhenRun = true

    @Parameter(title: "Persona") internal var persona: String

    @MainActor
    internal func perform() async throws -> some IntentResult {
        CallRequests.request(persona)
        return .result()
    }
}

internal struct CooeeShortcuts: AppShortcutsProvider {
    internal static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: CallPersonaIntent(), phrases: ["Call a persona in \(.applicationName)"], shortTitle: "Call",
            systemImageName: "phone.fill")
    }
}
