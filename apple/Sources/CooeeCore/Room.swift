import Foundation

public enum MemberState: String, Sendable, Equatable {
    case waiting
    case upNext = "up next"
    case replying, replied, passed
}

public struct RoomRound: Sendable, Equatable {
    public static let seats = 2...5
    public static let turns = 6
    public static let deadline = 600

    public private(set) var turn = 0
    public private(set) var since: Date?
    public private(set) var passed: [String] = []
    private var states: [String: MemberState] = [:]
    private var current: String?

    public var running: Bool { since != nil }

    public init() {}

    public func state(_ member: String) -> MemberState { states[member] ?? .waiting }

    public mutating func asked(_ members: [String], text: String, when: Date) {
        let next: MemberState = text.contains("@") ? .waiting : .upNext
        states = Dictionary(uniqueKeysWithValues: members.map { ($0, $0 == current ? .replying : next) })
        turn = 0
        since = when
        passed = []
    }

    public mutating func began(_ member: String) {
        settle()
        turn += 1
        current = member
        states[member] = .replying
    }

    public mutating func replied(_ member: String) {
        states[member] = .replied
        if member == current { current = nil }
    }

    public mutating func ended(_ reason: String) {
        guard reason != "cap" else { return }
        if reason == "done" {
            settle()
        } else if let current {
            states[current] = .waiting
        }
        current = nil
        since = nil
        states = states.mapValues { $0 == .upNext ? .waiting : $0 }
    }

    private mutating func settle() {
        guard let current, states[current] == .replying else { return }
        states[current] = .passed
        passed.append(current)
        self.current = nil
    }
}

public enum RoomOrder {
    public static func move(_ members: [String], _ member: String, to place: Int) -> [String] {
        guard let from = members.firstIndex(of: member) else { return members }
        var moved = members
        moved.remove(at: from)
        moved.insert(member, at: min(max(place, 0), moved.count))
        return moved
    }

    public static func ordinal(_ place: Int) -> String {
        let suffix = (11...13).contains(place % 100) ? "th" : ["th", "st", "nd", "rd"][place % 10 < 4 ? place % 10 : 0]
        return "\(place)\(suffix)"
    }
}

public enum RoomItems {
    public static func build(_ items: [APIItem], names: [String: String], last: RoomRound) -> [Item] {
        var out: [Item] = []
        var turns = 0
        for item in items {
            if item.kind == "you" {
                if !out.isEmpty { out.append(.divider(id: -item.id, text: ended(turns, passed: []))) }
                turns = 0
            }
            guard item.kind == "agent", let speaker = item.speaker, !speaker.isEmpty else {
                ChatItems.append(item, to: &out)
                continue
            }
            turns += 1
            let markdown = Markdown.blocks(item.text ?? "")
            out.append(.said(id: item.id, speaker: speaker, time: Clock.time(item.time), markdown: markdown))
        }
        if !last.running && last.turn > 0 {
            out.append(.divider(id: Int.min, text: ended(last.turn, passed: last.passed.map { names[$0] ?? $0 })))
        }
        return out
    }

    public static func ended(_ turns: Int, passed: [String]) -> String {
        (["round ended", turns == 1 ? "1 turn" : "\(turns) turns"] + passed.map { "\($0) passed" })
            .joined(separator: " · ")
    }
}

public enum Clock {
    public static func time(_ milliseconds: Int64, in zone: TimeZone = .current) -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm"
        formatter.timeZone = zone
        return formatter.string(from: Date(timeIntervalSince1970: Double(milliseconds) / 1000))
    }

    public static func round(_ seconds: Int) -> String {
        let seconds = max(0, seconds)
        return "\(seconds / 60):\(String(format: "%02d", seconds % 60))"
    }
}
