import AppIntents
import ComposableArchitecture
import CooeeCore
import CooeeFeatures

/// The surfaces' intents (#662), for the app target's `AppIntentsPackage`.
public struct SurfaceIntents: AppIntentsPackage {
    public static var includedPackages: [any AppIntentsPackage.Type] { [FeatureIntents.self] }
}

/// A persona, over the App Group copy the app keeps.
public struct PersonaEntity: AppEntity {
    public static let typeDisplayRepresentation: TypeDisplayRepresentation = "Persona"
    public static let defaultQuery = PersonaQuery()

    public let id: String
    public let name: String

    public var displayRepresentation: DisplayRepresentation { DisplayRepresentation(title: "\(name)") }

    public init(id: String, name: String) {
        self.id = id
        self.name = name
    }
}

public struct PersonaQuery: EntityStringQuery {
    @ComposableArchitecture.Dependency(\.surface) private var surface

    public init() {}

    private var all: [PersonaEntity] {
        (surface.cached()?.personas ?? []).map { PersonaEntity(id: $0.id, name: $0.name) }
    }

    public func entities(for identifiers: [String]) async throws -> [PersonaEntity] {
        all.filter { identifiers.contains($0.id) }
    }

    public func entities(matching string: String) async throws -> [PersonaEntity] {
        all.filter { $0.name.localizedCaseInsensitiveContains(string) }
    }

    public func suggestedEntities() async throws -> [PersonaEntity] { all }
}

/// A thread, over the App Group copy the app keeps.
public struct ThreadEntity: AppEntity {
    public static let typeDisplayRepresentation: TypeDisplayRepresentation = "Thread"
    public static let defaultQuery = ThreadQuery()

    public let id: String
    public let title: String
    public let persona: String

    public var displayRepresentation: DisplayRepresentation { DisplayRepresentation(title: "\(title)") }

    public init(_ row: GlanceRow) {
        id = row.thread
        title = row.title
        persona = row.persona
    }
}

public struct ThreadQuery: EntityStringQuery {
    @ComposableArchitecture.Dependency(\.surface) private var surface

    public init() {}

    private var all: [ThreadEntity] {
        (surface.cached()?.threads ?? []).map(ThreadEntity.init)
    }

    public func entities(for identifiers: [String]) async throws -> [ThreadEntity] {
        all.filter { identifiers.contains($0.id) }
    }

    public func entities(matching string: String) async throws -> [ThreadEntity] {
        all.filter { $0.title.localizedCaseInsensitiveContains(string) }
    }

    public func suggestedEntities() async throws -> [ThreadEntity] { all }
}

/// What needs me (the Action button, q1461): opens the top ask, else the top blocked lane, else Home.
public struct OpenNeedsYouIntent: AppIntent {
    public static let title: LocalizedStringResource = "What needs me"
    public static let openAppWhenRun = true

    @ComposableArchitecture.Dependency(\.surface) private var surface

    public init() {}

    @MainActor public func perform() async throws -> some IntentResult {
        if let top = surface.cached()?.glance.needs.first { await surface.open(.thread(top.thread)) }
        return .result()
    }
}

/// Opens a thread or its ask in the app; nothing is answered from outside it (q1310).
public struct OpenAskIntent: AppIntent {
    public static let title: LocalizedStringResource = "Open a thread"
    public static let openAppWhenRun = true

    @Parameter(title: "Thread") public var thread: ThreadEntity

    @ComposableArchitecture.Dependency(\.surface) private var surface

    public init() {}

    @MainActor public func perform() async throws -> some IntentResult {
        await surface.open(.thread(thread.id))
        return .result()
    }
}

/// A new chat with a persona, in the app.
public struct StartChatIntent: AppIntent {
    public static let title: LocalizedStringResource = "New chat"
    public static let openAppWhenRun = true

    @Parameter(title: "Persona") public var persona: PersonaEntity

    @ComposableArchitecture.Dependency(\.surface) private var surface

    public init() {}

    @MainActor public func perform() async throws -> some IntentResult {
        await surface.open(.start(persona.id))
        return .result()
    }
}
