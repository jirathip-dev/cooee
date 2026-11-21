import Blocks
import Foundation

public typealias JSONValue = Blocks.JSONValue
public typealias BlockAction = Blocks.BlockAction
public typealias Relative = Blocks.Relative

/// A screen kept live by the server's ops, applied by the blocks package; the badge count and dot stay here.
public struct LiveScreen: Sendable, Equatable {
    public private(set) var root: JSONValue
    public private(set) var seq = 0
    public private(set) var count: Int?
    public private(set) var dot = false

    public init(_ root: JSONValue) {
        self.root = root
    }

    public mutating func apply(event: JSONValue) {
        if let seq = event["seq"]?.number { self.seq = Int(seq) }
        for change in event["ops"]?.items ?? [] { apply(change) }
    }

    public mutating func apply(_ change: JSONValue) {
        var change = change
        let deck = change["screen"]?.isDeck ?? root.isDeck
        if let cards = change["screen"], deck { change = .object(change.fields.merging(["screen": cards.dealt]) { $1 }) }
        guard let op = try? change.decode(Blocks.Op.self) else { return }
        switch op {
        case .screen(let screen): root = (try? JSONValue(screen.screen)).map { $0.opened(deck) } ?? root
        case .badge(let badge): count = Int(badge.count)
        case .dot(let dot): self.dot = dot.on
        default:
            let tree = deck ? root.dealt : root
            guard var block = try? tree.decode(Blocks.Block.self), (try? block.apply([op])) != nil else { return }
            root = (try? JSONValue(block)).map { $0.opened(deck) } ?? root
        }
    }
}

extension JSONValue {
    fileprivate var isDeck: Bool {
        guard case .array = self else { return false }
        return true
    }

    fileprivate var dealt: Self { .object(["type": .string("screen"), "blocks": self]) }

    fileprivate func opened(_ deck: Bool) -> Self { deck ? .array(list("blocks")) : self }
}

extension BlockAction {
    public var answer: (option: String, value: String?)? {
        let other = values["other"]?.string.flatMap { $0.isEmpty ? nil : $0 }
        guard let option = other ?? values["choice"]?.string else { return nil }
        let rest = values.filter { $0.key != "choice" && $0.key != "other" }.sorted { $0.key < $1.key }
        return (option, rest.compactMap(\.value.string).first)
    }
}

extension JSONValue {
    public func text(at now: Date) -> String? {
        switch self {
        case .string(let string): return string
        case .number(let number): return number.formatted()
        case .object:
            if let since = self["since"]?.date { return Relative.text(now.timeIntervalSince(since)) }
            return self["until"]?.date.map { Relative.text($0.timeIntervalSince(now)) }
        default: return nil
        }
    }

    public var date: Date? { string.flatMap { try? Date($0, strategy: .iso8601) } }

    public init(json: String) {
        self = (try? JSONDecoder().decode(Self.self, from: Data(json.utf8))) ?? .null
    }
}

extension Relative {
    public static func stamp(in seconds: TimeInterval, from now: Date = .now) -> String {
        now.addingTimeInterval(seconds).ISO8601Format()
    }
}
