import Blocks
import Foundation

public typealias LayoutBroken = Blocks.LayoutBroken
public typealias LayoutContent = Blocks.LayoutContent
public typealias LayoutNode = Blocks.LayoutNode
public typealias LayoutPlace = Blocks.LayoutPlace
public typealias LayoutTransition = Blocks.LayoutTransition
public typealias Placement = Blocks.Placement
public typealias SizeClass = Blocks.SizeClass

/// Who serves a screen: cooee itself or an installed plugin.
public enum LayoutSource: Sendable, Equatable {
    case core
    case plugin(String)

    /// The source a layout node names: `core` or a plugin's name.
    public init(_ name: String) { self = name == "core" ? .core : .plugin(name) }

    /// The name a screen's key and a refusal use: `core` or the plugin's.
    public var name: String {
        switch self {
        case .core: "core"
        case .plugin(let name): name
        }
    }
}

/// cooee's `layout.json`, checked by `Blocks.Layout` with cooee's contract, and the etag it was served with.
public struct Layout: Sendable, Equatable {
    /// cooee's components beside `nav`, and the two screens no file may place.
    public static let contract = Blocks.Layout.Contract(
        components: [
            .init(kind: "sidebar", slots: ["top", "list"], config: ["width", "pinned"]),
            .init(kind: "chat", slots: ["strip", "cards"], config: ["composer", "stop", "back"]),
            .init(kind: "composer", config: ["attach", "queue", "hint"]),
            .init(kind: "terminal", config: ["rows", "cols", "machine"]),
            .init(kind: "scanner"),
            .init(kind: "menu-host", slots: ["rows"]),
            .init(kind: "settings", config: ["pages"]),
        ],
        locked: ["/screens/pairing", "/screens/reset-layout"])

    public let checked: Blocks.Layout
    public let etag: String

    /// `json` checked with cooee's contract.
    public init(json: Data, etag: String) throws(LayoutBroken) {
        checked = try Blocks.Layout(json: json, contract: Self.contract)
        self.etag = etag
    }

    public var root: LayoutNode<Placement> { checked.root }

    /// The tree with one place per node for `size`.
    public func resolved(for size: SizeClass) -> LayoutNode<LayoutPlace> { checked.resolved(for: size) }
}
