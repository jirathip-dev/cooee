import Foundation

/// Whose topics a strip lists (#517): a persona's threads, or the rooms seating one room's members.
public enum TopicsOf: Equatable, Sendable {
    case persona(String)
    case room(String)
}

/// A persona's topics (#515): its threads, in the order taken when its chat opened.
public struct Topics: Equatable, Sendable {
    public static let chips = 6

    public private(set) var rows: [ThreadRow] = []
    private var order: [String] = []

    public var open: [ThreadRow] { ordered.filter { !$0.archived } }
    public var archived: [ThreadRow] { ordered.filter(\.archived) }
    public var shown: [ThreadRow] { Array(open.prefix(Self.chips)) }
    public var folded: Int { max(0, open.count - Self.chips) }

    public var folders: [String] {
        var seen: Set<String> = []
        return rows.sorted { $0.updated > $1.updated }.compactMap(\.folder)
            .filter { !$0.isEmpty && seen.insert($0).inserted }
    }

    private var ordered: [ThreadRow] { order.compactMap { id in rows.first { $0.id == id } } }

    public init(_ rows: [ThreadRow] = []) {
        load(rows)
    }

    public mutating func load(_ rows: [ThreadRow]) {
        let fresh = rows.filter { !order.contains($0.id) }.sorted { $0.updated > $1.updated }.map(\.id)
        order = fresh + order.filter { id in rows.contains { $0.id == id } }
        self.rows = rows
    }

    public func row(_ id: String) -> ThreadRow? { rows.first { $0.id == id } }

    public mutating func retitle(_ id: String, _ title: String) {
        guard let index = rows.firstIndex(where: { $0.id == id }) else { return }
        rows[index].title = title
    }

    public mutating func archive(_ id: String, _ archived: Bool) {
        guard let index = rows.firstIndex(where: { $0.id == id }) else { return }
        rows[index].archived = archived
    }

    public static func found(_ rows: [ThreadRow], _ search: String) -> [ThreadRow] {
        let words = search.trimmingCharacters(in: .whitespaces)
        guard !words.isEmpty else { return rows }
        return rows.filter { row in
            [row.title, row.folder ?? "", row.last].contains { $0.localizedCaseInsensitiveContains(words) }
        }
    }

    public static func name(_ folder: String) -> String { (folder as NSString).lastPathComponent }

    public static func place(_ folder: String) -> String {
        let parts = folder.split(separator: "/", omittingEmptySubsequences: true)
        guard parts.count >= 2, ["Users", "home"].contains(parts[0]) else { return folder }
        return (["~"] + parts.dropFirst(2)).joined(separator: "/")
    }
}
