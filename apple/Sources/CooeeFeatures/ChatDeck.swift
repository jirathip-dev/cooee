import ComposableArchitecture
import CooeeAPI
import CooeeCore

/// A persona's plugin chat cards (#377): each `chat-cards` place's live screen, keyed by plugin and card id.
public struct PluginDeck: Equatable, Sendable {
    public struct Card: Equatable, Sendable {
        public let plugin: String
        public let block: JSONValue
    }

    private struct Key: Equatable, Sendable {
        fileprivate let plugin: String
        fileprivate let card: String
    }

    private struct Settled: Equatable, Sendable {
        fileprivate let card: Card
        fileprivate let was: JSONValue
    }

    private static let base = Int.min / 4
    public private(set) var sources: [Place] = []
    private var screens: [String: LiveScreen] = [:]
    private var keys: [Key] = []
    private var settled: [Int: Settled] = [:]

    public init() {}

    public var items: [Item] {
        keys.indices.compactMap { index in
            shown(index).map { .card(id: Self.base + index, poster: $0.plugin, block: $0.block) }
        }
    }

    private func shown(_ index: Int) -> Card? {
        let now = live(keys[index])
        guard let done = settled[index], now == nil || now?.block == done.was else { return now }
        return done.card
    }

    public func needsYou(_ plugin: String) -> Int {
        cards(plugin).filter { $0.string("kind") != "later" }.count
    }

    internal func card(_ item: Int) -> Card? {
        let index = item - Self.base
        guard keys.indices.contains(index), let now = live(keys[index]) else { return nil }
        return shown(index) == now ? now : nil
    }

    internal func holds(_ item: Int) -> Bool { keys.indices.contains(item - Self.base) }

    internal mutating func follow(_ places: [Place]) { sources = places }

    internal mutating func heard(_ plugin: String, _ event: JSONValue) {
        var screen = screens[plugin] ?? LiveScreen(.array([]))
        screen.apply(event: event)
        screens[plugin] = screen
        for card in cards(plugin) {
            let key = Key(plugin: plugin, card: card.id ?? "")
            if !keys.contains(key) { keys.append(key) }
        }
    }

    internal mutating func played(_ item: Int, _ reply: JSONValue, line: String?) {
        let index = item - Self.base
        guard keys.indices.contains(index) else { return }
        let was = live(keys[index])
        heard(keys[index].plugin, reply)
        guard let line, let was else { return }
        let text: JSONValue = .object(["type": .string("text"), "markdown": .string(line)])
        var block = was.block.fields.filter { ["type", "id", "kind", "title"].contains($0.key) }
        block["tone"] = .string("mute")
        block["blocks"] = .array([text])
        settled[index] = Settled(card: Card(plugin: was.plugin, block: .object(block)), was: was.block)
    }

    private func cards(_ plugin: String) -> [JSONValue] {
        let root = screens[plugin]?.root ?? .null
        return root.items.isEmpty ? root.list("blocks") : root.items
    }

    private func live(_ key: Key) -> Card? {
        cards(key.plugin).first { $0.id ?? "" == key.card }.map { Card(plugin: key.plugin, block: $0) }
    }
}

extension ChatFeature.State {
    public var chipCounts: [Int?] {
        places.enumerated().map { index, place in
            let first = places.firstIndex { $0.plugin == place.plugin } == index
            let waiting = deck.needsYou(place.plugin)
            return strips[place.id]?.count ?? place.count ?? (first && waiting > 0 ? waiting : nil)
        }
    }
}

extension ChatFeature {
    internal static func deal(
        _ places: [Place], _ plugins: PluginsClient, _ clock: any _Concurrency.Clock<Duration>, _ server: Server,
        _ send: Send<Action>
    ) async {
        await withTaskGroup(of: Void.self) { group in
            for place in places {
                group.addTask {
                    while !Task.isCancelled {
                        await follow(place, plugins, server, send)
                        try? await clock.sleep(for: .seconds(5))
                    }
                }
            }
        }
    }

    private static func follow(_ place: Place, _ plugins: PluginsClient, _ server: Server, _ send: Send<Action>) async {
        let source = LayoutSource.plugin(place.plugin)
        do {
            for try await event in plugins.events(server, source, place.screen + "/stream") {
                await send(place.cards ? .cards(.heard(place.plugin, event)) : .plugins(.counted(place.id, event)))
            }
        } catch {
            guard place.cards, let screen = try? await plugins.screen(server, source, place.screen) else { return }
            let whole: JSONValue = .object(["op": .string("screen"), "screen": screen])
            await send(.cards(.heard(place.plugin, .object(["ops": .array([whole])]))))
        }
    }

    internal func play(_ state: inout State, _ item: Int, _ action: BlockAction) -> Effect<Action> {
        guard let card = state.deck.card(item), !state.answering.contains(item) else { return .none }
        state.answering.insert(item)
        let (plugins, server) = (pluginAPI, state.server)
        let settles = action.action != "later" && card.block.string("kind") != "later"
        return .run { send in
            let reply = try await plugins.act(server, .plugin(card.plugin), "cards", action)
            let line = "✓ " + (reply.string("toast") ?? action.action)
            await send(.cards(.played(item, reply, settles ? line : nil)))
        } catch: { error, send in
            guard let refused = error as? APIError, refused.status == 409 else {
                return await send(.cards(.answered(item, error.localizedDescription)))
            }
            await send(.cards(.played(item, .null, refused.text.isEmpty ? "stale" : refused.text)))
        }
    }
}
