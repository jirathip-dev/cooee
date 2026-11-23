import AppIntents
import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import CooeeUI

/// Tell a persona something by voice: the snippet confirms before sending, to the persona's latest thread unless
/// one is named.
public struct SendToThreadIntent: AppIntent {
    public static let title: LocalizedStringResource = "Tell a persona"

    @Parameter(title: "Persona") public var persona: PersonaEntity
    @Parameter(title: "Message") public var text: String
    @Parameter(title: "Thread") public var thread: ThreadEntity?
    @ComposableArchitecture.Dependency(\.surface) private var surface
    @ComposableArchitecture.Dependency(\.serverStore) private var servers
    @ComposableArchitecture.Dependency(\.threads) private var threads

    public init() {}

    public func perform() async throws -> some IntentResult {
        let cache = surface.cached()
        let named = thread.flatMap { thread in cache?.threads.first { $0.thread == thread.id } }
        let row = named ?? cache?.latest(of: persona.id)
        let place = row.map { [$0.title, $0.machine, named == nil ? "the latest thread" : ""] } ?? ["a new chat"]
        let who = Persona.of(persona.id, persona.name, .idle, in: cache?.personas ?? [])
        let (line, words, style) = (place.filter { !$0.isEmpty }.joined(separator: " · "), text, cache?.style)
        try await requestConfirmation(
            conditions: [], actionName: .send, dialog: "Send to \(persona.name)?", showDialogAsPrompt: true
        ) { SendSnippet(persona: who, place: line, text: words, style: style) }
        guard let server = await servers.load() else { return .result() }
        if let row {
            try await threads.send(server, row.thread, text, [], false)
        } else if let api = cache?.personas.first(where: { $0.id == persona.id }) {
            _ = try await threads.start(server, api, "", text, false)
        }
        return .result()
    }
}
