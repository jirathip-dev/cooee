import Foundation

/// The two widths a layout places for: iPhone is `compact`, iPad and Mac are `wide`.
public enum SizeClass: Sendable {
    case compact, wide
}

/// Where a node sits once its size class is known.
public enum LayoutPlace: String, Sendable {
    case push, sheet, panel, column, strip, cards, menu, inline, hidden
}

/// A node's place on each size class; a size the file leaves out inherits the other.
public struct Placement: Sendable, Equatable {
    public let compact: LayoutPlace
    public let wide: LayoutPlace

    /// The place on one size class.
    public func on(_ size: SizeClass) -> LayoutPlace { size == .compact ? compact : wide }
}

/// Who serves a screen: cooee itself or an installed plugin.
public enum LayoutSource: Sendable, Equatable {
    case core
    case plugin(String)

    internal var name: String {
        switch self {
        case .core: "core"
        case .plugin(let name): name
        }
    }
}

/// A native piece the layout places and configures; its contract is fixed here, never by the file.
public enum ComponentKind: String, Sendable {
    case nav, sidebar, chat, composer, terminal, scanner
    case menuHost = "menu-host"
    case settings

    internal var slots: Set<String> {
        switch self {
        case .nav: ["side", "main", "menu"]
        case .sidebar: ["top", "list"]
        case .chat: ["strip", "cards"]
        case .menuHost: ["rows"]
        case .composer, .terminal, .scanner, .settings: []
        }
    }

    internal var config: Set<String> {
        switch self {
        case .sidebar: ["width", "pinned"]
        case .chat: ["composer", "stop", "back"]
        case .composer: ["attach", "queue", "hint"]
        case .terminal: ["rows", "cols", "machine"]
        case .settings: ["pages"]
        case .nav, .scanner, .menuHost: []
        }
    }
}

/// What a node draws: a screen of blocks from a source, or a component.
public enum LayoutContent: Sendable, Equatable {
    case screen(LayoutSource, path: String)
    case component(ComponentKind)

    internal var kind: ComponentKind? {
        guard case .component(let kind) = self else { return nil }
        return kind
    }

    internal var defaultId: String {
        switch self {
        case .screen(let source, let path): "\(source.name):\(path)"
        case .component(let kind): kind.rawValue
        }
    }
}

/// One node of the tree: placed by `Placement` as decoded, by `LayoutPlace` once resolved.
public struct LayoutNode<Place: Sendable & Equatable>: Sendable, Equatable {
    public let id: String
    public let content: LayoutContent
    public let label: String?
    public let place: Place
    public let config: [String: JSONValue]
    public let slots: [String: [Self]]
    public let locked: Bool

    internal func appending(_ child: Self, to slot: String) -> Self {
        var slots = slots
        slots[slot, default: []].append(child)
        return Self(
            id: id, content: content, label: label, place: place, config: config, slots: slots, locked: locked)
    }

    internal func map<Other>(_ resolve: (Place) -> Other) -> LayoutNode<Other> {
        LayoutNode<Other>(
            id: id, content: content, label: label, place: resolve(place), config: config,
            slots: slots.mapValues { $0.map { $0.map(resolve) } }, locked: locked)
    }
}

/// Why a `layout.json` was refused, with the JSON path of the node at fault.
public enum LayoutError: Error, Equatable {
    case format(Int)
    case json(String)
    case unknownType(path: String, type: String)
    case badScreen(path: String)
    case unknownSlot(path: String, slot: String)
    case unknownConfig(path: String, key: String)
    case badPlace(path: String, place: String)
    case duplicateId(String)
    case tooDeep(path: String)
    case tooMany(Int)
}

/// The owner's `layout.json`, decoded and validated, with the two locked screens always in it.
public struct Layout: Sendable, Equatable {
    internal static let knownFormat = 1
    internal static let maxBytes = 64 * 1024
    internal static let pairing = locked("pairing", place: .push)
    internal static let resetLayout = locked("reset-layout", place: .menu)

    public let format: Int
    public let theme: String?
    public let root: LayoutNode<Placement>
    public let etag: String

    /// Decodes and validates `json`, drops any node naming a locked screen and appends the locked two.
    public init(json: Data, etag: String) throws(LayoutError) {
        guard json.count <= Self.maxBytes else { throw .json("over \(Self.maxBytes) bytes") }
        let file: JSONValue
        do {
            file = try JSONDecoder().decode(JSONValue.self, from: json)
        } catch {
            throw .json(error.localizedDescription)
        }
        let format = Int(file["format"]?.number ?? 0)
        guard (1...Self.knownFormat).contains(format) else { throw .format(format) }
        var decoder = LayoutDecoder()
        let rootJSON = file["root"] ?? .null
        guard let root = try decoder.node(rootJSON, at: "root", depth: 1), root.content == .component(.nav) else {
            throw .unknownType(path: "root", type: rootJSON.type)
        }
        self.format = format
        self.theme = file.string("theme")
        self.etag = etag
        self.root = root.appending(Self.pairing, to: "main").appending(Self.resetLayout, to: "menu")
    }

    /// The tree with one place per node for `size`.
    public func resolved(for size: SizeClass) -> LayoutNode<LayoutPlace> { root.map { $0.on(size) } }

    private static func locked(_ name: String, place: LayoutPlace) -> LayoutNode<Placement> {
        LayoutNode(
            id: name, content: .screen(.core, path: "/screens/\(name)"), label: nil,
            place: Placement(compact: place, wide: place), config: [:], slots: [:], locked: true)
    }
}

private struct LayoutDecoder {
    private static let maxDepth = 6
    private static let maxNodes = 200
    private static let lockedPaths: Set<String> = [Layout.pairing, Layout.resetLayout].reduce(into: []) {
        if case .screen(_, let path) = $1.content { $0.insert(path) }
    }

    private var ids: Set<String> = [Layout.pairing.id, Layout.resetLayout.id]
    private var count = 0

    fileprivate mutating func node(
        _ json: JSONValue, at path: String, depth: Int
    ) throws(LayoutError) -> LayoutNode<Placement>? {
        guard depth <= Self.maxDepth else { throw .tooDeep(path: path) }
        count += 1
        guard count <= Self.maxNodes else { throw .tooMany(Self.maxNodes) }
        let content = try Self.content(json, at: path)
        if case .screen(_, let screen) = content, Self.lockedPaths.contains(screen) { return nil }
        let id = json.string("id") ?? content.defaultId
        guard ids.insert(id).inserted else { throw .duplicateId(id) }
        return LayoutNode(
            id: id, content: content, label: json.string("label"), place: try Self.placement(json["place"], at: path),
            config: try Self.config(json, of: content, at: path),
            slots: try slots(json, of: content, at: path, depth: depth), locked: false)
    }

    private mutating func slots(
        _ json: JSONValue, of content: LayoutContent, at path: String, depth: Int
    ) throws(LayoutError) -> [String: [LayoutNode<Placement>]] {
        let slots = json["slots"]?.fields ?? [:]
        let allowed = content.kind?.slots ?? []
        var decoded: [String: [LayoutNode<Placement>]] = [:]
        for name in slots.keys.sorted() {
            guard allowed.contains(name) else { throw .unknownSlot(path: path, slot: name) }
            var nodes: [LayoutNode<Placement>] = []
            for (index, child) in (slots[name]?.items ?? []).enumerated() {
                if let node = try node(child, at: "\(path).slots.\(name)[\(index)]", depth: depth + 1) {
                    nodes.append(node)
                }
            }
            decoded[name] = nodes
        }
        return decoded
    }

    private static func content(_ json: JSONValue, at path: String) throws(LayoutError) -> LayoutContent {
        guard json.type == "screen" else {
            guard let kind = ComponentKind(rawValue: json.type) else { throw .unknownType(path: path, type: json.type) }
            return .component(kind)
        }
        guard let source = json.string("source"), !source.isEmpty, let screen = json.string("path"),
            screen.hasPrefix("/screens/")
        else { throw .badScreen(path: path) }
        return .screen(source == "core" ? .core : .plugin(source), path: screen)
    }

    private static func config(
        _ json: JSONValue, of content: LayoutContent, at path: String
    ) throws(LayoutError) -> [String: JSONValue] {
        let config = json["config"]?.fields ?? [:]
        let allowed = content.kind?.config ?? []
        if let key = config.keys.sorted().first(where: { !allowed.contains($0) }) {
            throw .unknownConfig(path: path, key: key)
        }
        return config
    }

    private static func placement(_ json: JSONValue?, at path: String) throws(LayoutError) -> Placement {
        switch json {
        case nil:
            return Placement(compact: .inline, wide: .inline)
        case .string(let both)?:
            let place = try place(both, at: path)
            return Placement(compact: place, wide: place)
        case let sizes?:
            let compact = sizes.string("compact") ?? sizes.string("wide")
            let wide = sizes.string("wide") ?? sizes.string("compact")
            return Placement(compact: try place(compact, at: path), wide: try place(wide, at: path))
        }
    }

    private static func place(_ raw: String?, at path: String) throws(LayoutError) -> LayoutPlace {
        guard let raw, let place = LayoutPlace(rawValue: raw) else { throw .badPlace(path: path, place: raw ?? "") }
        return place
    }
}
