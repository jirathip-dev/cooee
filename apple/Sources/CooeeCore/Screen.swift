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
        switch change["op"]?.string {
        case "screen": root = change["screen"] ?? root
        case "badge": count = change["count"]?.number.map { Int($0) }
        case "dot": dot = change["on"]?.bool ?? false
        default: patch(change)
        }
    }

    private mutating func patch(_ change: JSONValue) {
        let id = change["id"]?.string
        switch change["op"]?.string {
        case "replace": edit(id) { _ in change["block"].map { [$0] } ?? [] }
        case "merge": edit(id) { [$0.merging(change["fields"]?.fields ?? [:])] }
        case "remove": edit(id) { _ in [] }
        case "insert": edit(change["into"]?.string) { [$0.inserting(change)] }
        default: break
        }
    }

    private mutating func edit(_ id: String?, _ change: (JSONValue) -> [JSONValue]) {
        guard let id else { return }
        root = JSONValue.array([root]).editing { $0.id == id ? change($0) : [$0] }.items.first ?? .null
    }
}

extension JSONValue {
    fileprivate func editing(_ edit: (Self) -> [Self]) -> Self {
        switch self {
        case .array(let items): .array(items.flatMap { edit($0.editing(edit)) })
        case .object(let fields): .object(fields.mapValues { $0.editing(edit) })
        default: self
        }
    }

    fileprivate func merging(_ more: [String: Self]) -> Self {
        .object(fields.merging(more) { _, new in new })
    }

    fileprivate func inserting(_ change: Self) -> Self {
        guard let block = change["block"] else { return self }
        let key = ["rows", "lanes", "rounds"].first { self[$0] != nil } ?? "blocks"
        var children = self[key]?.items ?? []
        let before = change["before"]?.string.flatMap { id in children.firstIndex { $0.id == id } }
        let after = change["after"]?.string.flatMap { id in children.firstIndex { $0.id == id } }
        children.insert(block, at: before ?? after.map { $0 + 1 } ?? children.count)
        return merging([key: .array(children)])
    }
}

public struct BlockAction: Sendable, Equatable {
    public let action: String
    public let values: [String: JSONValue]

    public init(action: String, values: [String: JSONValue]) {
        self.action = action
        self.values = values
    }

    public init?(action: String?, source: JSONValue, values: [String: JSONValue]) {
        let tap = source["tap"] ?? .null
        guard let name = action ?? source.string("action") ?? tap.string("action") else { return nil }
        let own =
            tap["values"]?.fields.merging(source["values"]?.fields ?? [:]) { $1 } ?? source["values"]?.fields ?? [:]
        self.init(action: name, values: own.merging(values) { $1 })
    }

    public var answer: (option: String, value: String?)? {
        let other = values["other"]?.string.flatMap { $0.isEmpty ? nil : $0 }
        guard let option = other ?? values["choice"]?.string else { return nil }
        let rest = values.filter { $0.key != "choice" && $0.key != "other" }.sorted { $0.key < $1.key }
        return (option, rest.compactMap(\.value.string).first)
    }
}

public struct Confirm: Sendable, Equatable {
    public let title: String
    public let message: String?
    public let accept: String
    public let destructive: Bool

    public init?(for source: JSONValue) {
        let destructive = source.string("style") == "destructive" || source.string("tone") == "bad"
        guard let confirm = source["confirm"] ?? (destructive ? .object([:]) : nil) else { return nil }
        let label = source.string("label")
        title = confirm.string("title") ?? label ?? ""
        message = confirm.string("message")
        accept = confirm.string("ok") ?? label ?? "OK"
        self.destructive = destructive || confirm["destructive"]?.bool == true
    }
}

extension JSONValue {
    /// A list's row ids once the row `moved` is dropped in the place of the row `target`.
    public func rowOrder(moving moved: String, onto target: String) -> [String] {
        var ids = list("rows").compactMap(\.id)
        guard let from = ids.firstIndex(of: moved), let place = ids.firstIndex(of: target) else { return ids }
        ids.remove(at: from)
        ids.insert(moved, at: place)
        return ids
    }
}
