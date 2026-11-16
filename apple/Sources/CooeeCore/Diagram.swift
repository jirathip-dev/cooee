import Foundation

/// A `diagram` block's Mermaid `stateDiagram-v2` subset, laid out as one top-down spine with two side columns (#213).
public struct Diagram: Sendable, Equatable {
    public struct Edge: Sendable, Equatable {
        public let from: String
        public let into: String
        public var both = false
    }

    public private(set) var order: [String] = []
    public private(set) var names: [String: String] = [:]
    public private(set) var spine: [String] = []
    public private(set) var row: [String: Int] = [:]
    public private(set) var side: [String: Int] = [:]
    public private(set) var optional: Set<String> = []
    public private(set) var edges: [Edge] = []
    private var parsed: [Edge] = []

    public var rows: Int { (row.values.max() ?? -1) + 1 }

    public func name(_ id: String) -> String { names[id] ?? id }

    public init(source: String) {
        for line in source.split(separator: "\n") { read(line.trimmingCharacters(in: .whitespaces)) }
        placeSpine()
        for id in order where side[id] == nil { placeBeside(id) }
        joinEdges()
    }

    private mutating func add(_ id: String) {
        if id != "[*]", !order.contains(id) { order.append(id) }
    }

    private mutating func read(_ line: String) {
        if let named = line.wholeMatch(of: #/state\s+"([^"]+)"\s+as\s+(\w+)/#) {
            names[String(named.2)] = String(named.1)
            add(String(named.2))
        } else if let plain = line.wholeMatch(of: #/state\s+(\w+)/#) {
            add(String(plain.1))
        } else if let edge = line.prefixMatch(of: #/(\[\*\]|\w+)\s*-->\s*(\[\*\]|\w+)/#) {
            add(String(edge.1))
            add(String(edge.2))
            parsed.append(Edge(from: String(edge.1), into: String(edge.2)))
        }
    }

    private var forward: [Edge] {
        parsed.filter { edge in
            guard let from = order.firstIndex(of: edge.from), let end = order.firstIndex(of: edge.into) else {
                return false
            }
            return end > from
        }
    }

    private mutating func placeSpine() {
        let forward = forward
        var length: [String: Int] = [:]
        var next: [String: String] = [:]
        for id in order.reversed() {
            for edge in forward where edge.from == id && length[edge.into, default: 1] + 1 > length[id, default: 1] {
                length[id] = length[edge.into, default: 1] + 1
                next[id] = edge.into
            }
        }
        var id = parsed.first { $0.from == "[*]" }?.into ?? order.first
        while let state = id, !spine.contains(state) {
            row[state] = spine.count
            side[state] = 0
            spine.append(state)
            id = next[state]
        }
        for edge in forward {
            guard let from = spine.firstIndex(of: edge.from), let end = spine.firstIndex(of: edge.into), end > from + 1
            else { continue }
            optional.formUnion(spine[(from + 1)..<end])
        }
    }

    private mutating func placeBeside(_ id: String) {
        let touching = parsed.compactMap { $0.from == id ? $0.into : $0.into == id ? $0.from : nil }
            .filter { side[$0] == 0 }.compactMap { row[$0] }
        let before = parsed.filter { $0.into == id && row[$0.from] != nil }.map(\.from)
        var (column, place) = (1, 0)
        if let first = touching.min() {
            place = first
            column = parsed.contains { $0.from == id && side[$0.into] == 0 } ? 1 : -1
        } else if let last = before.compactMap({ row[$0] }).max() {
            place = last + 1
            column = before.compactMap { side[$0] }.first { $0 != 0 } ?? 1
        }
        while side.contains(where: { $0.value == column && row[$0.key] == place }) { place += 1 }
        row[id] = place
        side[id] = column
    }

    private mutating func joinEdges() {
        for edge in parsed {
            guard let from = row[edge.from], let end = row[edge.into] else { continue }
            if side[edge.from] == 0 && side[edge.into] == 0 && abs(from - end) > 1 { continue }
            if let twin = edges.firstIndex(where: { $0.from == edge.into && $0.into == edge.from }) {
                edges[twin].both = true
            } else {
                edges.append(Edge(from: edge.from, into: edge.into))
            }
        }
    }
}
