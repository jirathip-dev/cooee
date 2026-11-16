import ComposableArchitecture
import CooeeAPI
import CooeeCore
import Foundation

@DependencyClient
public struct LayoutClient: Sendable {
    public var fetch: @Sendable (_ server: Server, _ etag: String) async throws -> LayoutAnswer?
    public var events: @Sendable (_ server: Server) -> AsyncThrowingStream<LayoutEvent, any Error> = { _ in .finished()
    }
    public var reset: @Sendable (_ server: Server) async throws -> LayoutAnswer
}

@DependencyClient
public struct LayoutStore: Sendable {
    public var load: @Sendable () throws -> Layout
    public var save: @Sendable (_ answer: LayoutAnswer?) -> Void

    /// The last good layout in `file`, else `bundled`; saving `nil` forgets it, so the next load is `bundled`.
    public static func onDisk(_ file: LayoutFile, bundled: Layout) -> Self {
        Self(
            load: { file.load().flatMap { try? Layout($0) } ?? bundled },
            save: { file.save($0) }
        )
    }
}

extension Layout {
    fileprivate init(_ answer: LayoutAnswer) throws(LayoutError) {
        do {
            try self.init(json: JSONEncoder().encode(answer.layout), etag: answer.etag)
        } catch let error as LayoutError {
            throw error
        } catch {
            throw .json(error.localizedDescription)
        }
    }
}

@Reducer
public struct LayoutFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public var server: Server?
        public var layout: Layout
        public var broken: String?
        public var screens: IdentifiedArrayOf<ScreenFeature.State> = []

        public init(server: Server?, layout: Layout) {
            self.server = server
            self.layout = layout
            place()
        }

        fileprivate mutating func place() {
            guard let server else { return }
            let fresh = layout.resolved(for: .wide).sidebarRows(on: .wide).compactMap { node -> ScreenFeature.State? in
                guard node.place == .column, case .screen(let source, let path) = node.content else { return nil }
                return screens[id: ScreenFeature.State.id(source, path)]
                    ?? ScreenFeature.State(server: server, source: source, path: path, label: node.label ?? "")
            }
            screens = IdentifiedArray(fresh, uniquingIDsWith: { first, _ in first })
        }
    }

    public enum Action: Sendable {
        case task
        case fetched(LayoutAnswer?)
        case event(LayoutEvent)
        case reset
        case resetDone(LayoutAnswer?)
        case screens(IdentifiedActionOf<ScreenFeature>)
    }

    private enum CancelID { case fetch }

    @Dependency(\.layoutClient) private var client
    @Dependency(\.layoutStore) private var store

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .task:
                return follow(state)
            case .fetched(let answer):
                if let answer { apply(&state, answer) }
            case .event(.changed(let etag)):
                state.broken = nil
                return etag == state.layout.etag ? .none : fetch(state)
            case .event(.broken(let line, let why)):
                state.broken = "layout.json line \(line): \(why) · last good layout kept"
            case .reset:
                return reset(state)
            case .resetDone(let answer?):
                apply(&state, answer)
            case .resetDone(nil):
                store.save(nil)
                state.layout = (try? store.load()) ?? state.layout
                state.place()
                state.broken = "reset here; the Mac's layout.json is unchanged until it's back"
            case .screens:
                break
            }
            return .none
        }
        .forEach(\.screens, action: \.screens) { ScreenFeature() }
    }

    private func apply(_ state: inout State, _ answer: LayoutAnswer) {
        do {
            state.layout = try Layout(answer)
            state.place()
            state.broken = answer.skipped?.first.map { "layout.json \($0) · not drawn" }
            store.save(answer)
        } catch {
            state.broken = "app can't read this layout: \(error) · last good layout kept"
        }
    }

    private func fetch(_ state: State) -> Effect<Action> {
        guard let server = state.server else { return .none }
        let (client, etag) = (client, state.layout.etag)
        return .run { send in
            await send(.fetched(try await client.fetch(server, etag)))
        } catch: { _, _ in
        }
        .cancellable(id: CancelID.fetch, cancelInFlight: true)
    }

    private func follow(_ state: State) -> Effect<Action> {
        guard let server = state.server else { return .none }
        let (client, etag) = (client, state.layout.etag)
        return .run { send in
            if let answer = try? await client.fetch(server, etag) { await send(.fetched(answer)) }
            for try await event in client.events(server) { await send(.event(event)) }
        } catch: { _, _ in
        }
    }

    private func reset(_ state: State) -> Effect<Action> {
        guard let server = state.server else { return .send(.resetDone(nil)) }
        let client = client
        return .run { send in
            await send(.resetDone(try await client.reset(server)))
        } catch: { _, send in
            await send(.resetDone(nil))
        }
    }
}

extension LayoutClient: TestDependencyKey {
    public static let testValue = LayoutClient()
}

extension LayoutStore: TestDependencyKey {
    public static let testValue = LayoutStore()
}

extension DependencyValues {
    public var layoutClient: LayoutClient {
        get { self[LayoutClient.self] }
        set { self[LayoutClient.self] = newValue }
    }

    public var layoutStore: LayoutStore {
        get { self[LayoutStore.self] }
        set { self[LayoutStore.self] = newValue }
    }
}
