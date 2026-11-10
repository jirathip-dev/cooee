import Foundation

public enum JSONValue: Sendable, Hashable, Codable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([Self])
    case object([String: Self])

    public init(from decoder: any Decoder) throws {
        let single = try decoder.singleValueContainer()
        if single.decodeNil() {
            self = .null
        } else if let bool = try? single.decode(Bool.self) {
            self = .bool(bool)
        } else if let number = try? single.decode(Double.self) {
            self = .number(number)
        } else if let string = try? single.decode(String.self) {
            self = .string(string)
        } else if let array = try? single.decode([Self].self) {
            self = .array(array)
        } else {
            self = .object(try single.decode([String: Self].self))
        }
    }

    public func encode(to encoder: any Encoder) throws {
        var single = encoder.singleValueContainer()
        switch self {
        case .null: try single.encodeNil()
        case .bool(let bool): try single.encode(bool)
        case .number(let number): try single.encode(number)
        case .string(let string): try single.encode(string)
        case .array(let array): try single.encode(array)
        case .object(let object): try single.encode(object)
        }
    }

    public subscript(key: String) -> Self? {
        guard case .object(let object) = self else { return nil }
        return object[key]
    }

    public var string: String? {
        guard case .string(let string) = self else { return nil }
        return string
    }

    public var bool: Bool? {
        guard case .bool(let bool) = self else { return nil }
        return bool
    }

    public var number: Double? {
        guard case .number(let number) = self else { return nil }
        return number
    }

    public var items: [Self] {
        guard case .array(let array) = self else { return [] }
        return array
    }

    public var fields: [String: Self] {
        guard case .object(let object) = self else { return [:] }
        return object
    }

    public var type: String { self["type"]?.string ?? "" }

    public func string(_ key: String) -> String? { self[key]?.string }

    public func list(_ key: String) -> [Self] { self[key]?.items ?? [] }

    public var fieldValues: [String: Self] {
        let own = list("fields").reduce(into: [String: Self]()) { values, field in
            if let name = field.string("name"), let value = field["value"] { values[name] = value }
        }
        return list("blocks").reduce(own) { $0.merging($1.fieldValues) { old, _ in old } }
    }

    public var id: String? { self["id"]?.string }

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
}

public enum Relative {
    public static func text(_ seconds: TimeInterval) -> String {
        let minutes = Int(max(0, seconds)) / 60
        if minutes < 1 { return "now" }
        if minutes < 60 { return "\(minutes)m" }
        if minutes < 24 * 60 { return "\(minutes / 60)h" }
        return "\(minutes / (24 * 60))d"
    }

    public static func filled(_ text: String, until: Date?, at now: Date) -> String {
        guard let until else { return text }
        return text.replacingOccurrences(of: "{rel}", with: Self.text(until.timeIntervalSince(now)))
    }
}
