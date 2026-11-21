import ComposableArchitecture
import CooeeCore
import Foundation

/// A persona or room chat's topics strip, its list sheet and New's folder (#515).
public struct TopicBar: Equatable, Sendable {
    public var topics = Topics()
    public var listing = false
    public var search = ""
    public var pressed: String?
    public var renaming: String?
    public var title = ""
    public var undo: String?
    public var folder: String?
    public var folders: FolderListing?
    public var find = ""

    public var recent: [String] {
        var seen: Set<String> = []
        return Array(((folder.map { [$0] } ?? []) + topics.folders).filter { seen.insert($0).inserted }.prefix(3))
    }

    public init(folder: String) {
        self.folder = ["", "scratch"].contains(folder) ? nil : folder
    }
}

extension ChatFeature.Action {
    @CasePathable
    public enum TopicAction: Sendable, Equatable {
        case load
        case loaded([ThreadRow])
        case open(String?)
        case list(Bool)
        case search(String)
        case press(String?)
        case rename(String?)
        case title(String)
        case retitle
        case archive(String)
        case undo
        case expired
        case restore(String)
        case folder(String?)
        case browse(String?)
        case browsed(FolderListing?)
        case find(String)
    }
}

extension ChatFeature {
    private enum Toast: Hashable { case undo }

    internal func topics(_ state: inout State, _ action: Action.TopicAction) -> Effect<Action> {
        guard var bar = state.bar else { return .none }
        defer { state.bar = bar }
        switch action {
        case .load: return load(state)
        case .loaded(let rows): bar.topics.load(rows)
        case .open(let id): return open(&bar, id, state)
        case .list(let shown): (bar.listing, bar.renaming, bar.search) = (shown, nil, "")
        case .search(let words): bar.search = words
        case .press(let id): bar.pressed = id
        default: return edit(&bar, action, state.server)
        }
        return .none
    }

    private func edit(_ bar: inout TopicBar, _ action: Action.TopicAction, _ server: Server) -> Effect<Action> {
        switch action {
        case .rename(let id):
            (bar.pressed, bar.renaming, bar.title) = (nil, id, id.flatMap(bar.topics.row)?.title ?? "")
            bar.listing = bar.listing || id != nil
        case .title(let title): bar.title = title
        case .retitle: return retitle(&bar, server)
        case .archive(let id): return archive(&bar, id, server)
        case .undo: return undo(&bar, server)
        case .expired: bar.undo = nil
        default: return folders(&bar, action, server)
        }
        return .none
    }

    private func folders(_ bar: inout TopicBar, _ action: Action.TopicAction, _ server: Server) -> Effect<Action> {
        switch action {
        case .folder(let folder): (bar.folder, bar.folders) = (folder == "scratch" ? nil : folder, nil)
        case .browsed(let listing): (bar.folders, bar.find) = (listing, "")
        case .find(let words): bar.find = words
        case .restore(let id): return keep(&bar, id, server)
        case .browse(let path):
            if let path, bar.folders?.holds(path) != true { return .none }
            let (personas, server) = (personas, server)
            return .run { send in
                await send(.topics(.browsed(try await personas.folders(server, path))))
            } catch: { error, send in
                await send(.failed(error.localizedDescription))
            }
        default: break
        }
        return .none
    }

    private func load(_ state: State) -> Effect<Action> {
        let owner: TopicsOf = state.room == nil ? .persona(state.persona.api.id) : .room(state.current)
        let (threads, server) = (threads, state.server)
        return .run { send in
            await send(.topics(.loaded(try await threads.topics(server, owner))))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }

    private func open(_ bar: inout TopicBar, _ id: String?, _ state: State) -> Effect<Action> {
        (bar.listing, bar.pressed, bar.renaming) = (false, nil, nil)
        if id == nil, let members = state.room?.members {
            let (threads, server) = (threads, state.server)
            return .run { send in
                await send(.delegate(.topic(try await threads.room(server, "", members))))
            } catch: { error, send in
                await send(.failed(error.localizedDescription))
            }
        }
        let target = id ?? "new:\(state.persona.api.id)"
        return target == state.current ? .none : .send(.delegate(.topic(target)))
    }

    private func retitle(_ bar: inout TopicBar, _ server: Server) -> Effect<Action> {
        let title = bar.title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let id = bar.renaming else { return .none }
        bar.renaming = nil
        guard !title.isEmpty, title != bar.topics.row(id)?.title else { return .none }
        bar.topics.retitle(id, title)
        let threads = threads
        return call { try await threads.retitle(server, id, title) }
    }

    private func archive(_ bar: inout TopicBar, _ id: String, _ server: Server) -> Effect<Action> {
        (bar.pressed, bar.undo) = (nil, id)
        bar.topics.archive(id, true)
        let (threads, clock) = (threads, clock)
        return .merge(
            call { try await threads.archive(server, id, true) },
            .run { send in
                try await clock.sleep(for: .seconds(5))
                await send(.topics(.expired))
            }
            .cancellable(id: Toast.undo, cancelInFlight: true)
        )
    }

    private func undo(_ bar: inout TopicBar, _ server: Server) -> Effect<Action> {
        guard let id = bar.undo else { return .none }
        bar.undo = nil
        return .merge(.cancel(id: Toast.undo), keep(&bar, id, server))
    }

    private func keep(_ bar: inout TopicBar, _ id: String, _ server: Server) -> Effect<Action> {
        bar.topics.archive(id, false)
        let threads = threads
        return call { try await threads.archive(server, id, false) }
    }

    private func call(_ work: @escaping @Sendable () async throws -> Void) -> Effect<Action> {
        .run { _ in
            try await work()
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}
