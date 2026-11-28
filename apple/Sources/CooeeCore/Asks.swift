import Foundation

/// A group of a chat's Asks (#852): a plugin manifest's `[[groups]]` entry, or one of cooee's own.
public struct AskGroup: Codable, Sendable, Equatable, Hashable {
    private enum CodingKeys: String, CodingKey { case id, label, urgent }

    public let id: String
    public let label: String
    public let urgent: Bool

    public init(id: String, label: String, urgent: Bool = false) {
        self.id = id
        self.label = label
        self.urgent = urgent
    }

    public init(from decoder: any Decoder) throws {
        let fields = try decoder.container(keyedBy: CodingKeys.self)
        id = try fields.decode(String.self, forKey: .id)
        label = try fields.decode(String.self, forKey: .label)
        urgent = try fields.decodeIfPresent(Bool.self, forKey: .urgent) ?? false
    }
}

/// One card of a chat's Asks: a thread item or a plugin's chat card that waits for the owner.
public struct OpenAsk: Sendable, Equatable, Identifiable {
    public let id: Int
    public let plugin: String?
    public let group: String
    public let text: String
    public let block: JSONValue
    public let time: Int64
    public var settled = false

    public init(id: Int, plugin: String?, group: String, text: String, block: JSONValue, time: Int64) {
        self.id = id
        self.plugin = plugin
        self.group = group
        self.text = text
        self.block = block
        self.time = time
    }
}

/// A heading of the Asks sheet and its cards, oldest first.
public struct AskSection: Sendable, Equatable, Identifiable {
    public let group: AskGroup
    public let asks: [OpenAsk]

    public var id: String { group.id }
}

/// The Asks a chat shows (#852): its sections in order, how many wait, and whether one is urgent.
public struct Asks: Sendable, Equatable {
    public static let approvals = AskGroup(id: "approvals", label: "Approvals")
    public static let checkIn = AskGroup(id: "check-in", label: "Check-in")

    public let sections: [AskSection]

    public var waiting: [OpenAsk] { sections.flatMap(\.asks).filter { !$0.settled } }
    public var count: Int { waiting.count }
    public var top: OpenAsk? { waiting.first }
    public var urgent: Bool { sections.contains { $0.group.urgent && $0.asks.contains { !$0.settled } } }

    /// Sorts `asks` by group: urgent ones first, then cooee's own, then each plugin's in its manifest's order.
    public init(_ asks: [OpenAsk], groups: [String: [AskGroup]]) {
        let own = [Self.approvals, Self.checkIn].map { (plugin: String?.none, group: $0) }
        let theirs = groups.keys.sorted().flatMap { name in groups[name, default: []].map { (Optional(name), $0) } }
        let listed = (own + theirs).map { (plugin: $0.0, group: $0.1) }
        let order = listed.filter(\.group.urgent) + listed.filter { !$0.group.urgent }
        var left = asks.sorted { $0.time < $1.time }
        var sections: [AskSection] = []
        for entry in order {
            let mine = left.filter { $0.plugin == entry.plugin && $0.group == entry.group.id }
            left.removeAll { mine.contains($0) }
            if !mine.isEmpty { sections.append(AskSection(group: entry.group, asks: mine)) }
        }
        for name in Set(left.map { $0.plugin ?? "" }).sorted() {
            let rest = left.filter { ($0.plugin ?? "") == name }
            sections.append(AskSection(group: AskGroup(id: "other-\(name)", label: name), asks: rest))
        }
        self.sections = sections
    }

    /// The thread's open approvals and plugin rows: an approval after a routine's message is a check-in finding.
    public static func waiting(in items: [APIItem]) -> [OpenAsk] {
        var routine = false
        var out: [OpenAsk] = []
        for item in items {
            if item.kind == "you" { routine = item.routine != nil }
            guard item.waits == true, let block = item.block else { continue }
            let row = item.kind == "row"
            let group = row ? item.group ?? "" : (routine ? checkIn : approvals).id
            let text = item.text ?? item.tool ?? block.string("title") ?? ""
            out.append(
                OpenAsk(
                    id: item.id, plugin: row ? item.plugin : nil, group: group, text: text, block: block,
                    time: item.time))
        }
        return out
    }
}
