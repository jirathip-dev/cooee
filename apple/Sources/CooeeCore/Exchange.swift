import Foundation

public struct Ask: Sendable, Equatable {
    public enum Outcome: String, Sendable {
        case asking, replied, timeout, offline, depth
        case notAllowed = "not_allowed"
    }

    public let persona: String
    public let name: String
    public let text: String
    public let thread: String
    public let outcome: Outcome
    public let seconds: Int
    public let note: String

    public var refused: Bool { [.notAllowed, .depth, .offline].contains(outcome) }

    public init(
        persona: String,
        name: String,
        text: String,
        thread: String,
        outcome: Outcome,
        seconds: Int = 0,
        note: String = ""
    ) {
        self.persona = persona
        self.name = name
        self.text = text
        self.thread = thread
        self.outcome = outcome
        self.seconds = seconds
        self.note = note
    }

    public init?(_ item: APIItem) {
        guard let persona = item.persona, let outcome = Outcome(rawValue: item.outcome ?? "") else { return nil }
        self.init(
            persona: persona, name: item.name ?? persona, text: item.text ?? "", thread: item.thread ?? "",
            outcome: outcome, seconds: item.secs ?? 0, note: item.note ?? ""
        )
    }

    /// The card's last line, as `asker` asked it at `depth` (nil: the asker's own chat).
    public func line(asker: String, depth: Int? = nil) -> String {
        switch outcome {
        case .asking: "asking…"
        case .replied:
            (["✓ replied in \(seconds)s"] + (note.isEmpty ? [] : ["with your note"])
                + (depth.map { ["depth \($0)"] } ?? []))
                .joined(separator: " · ")
        case .timeout: "timed out after \(seconds)s"
        case .notAllowed: "Not sent: \(asker) may not ask \(name)"
        case .depth: "Not sent: \(name) is past depth \(Exchange.depth)"
        case .offline: "Not sent: \(name) is offline"
        }
    }
}

public struct Exchange: Sendable, Equatable {
    public static let depth = 2

    public let asker: APIPersona
    public let target: APIPersona
    public let from: String
    public let title: String
    public let depth: Int
    public let chat: String?

    public var meter: String {
        let more = depth < Self.depth ? " · \(target.name) could ask one more" : ""
        return "depth \(depth) of \(Self.depth)\(more)"
    }

    public init(asker: APIPersona, target: APIPersona, from: String, title: String, depth: Int, chat: String?) {
        self.asker = asker
        self.target = target
        self.from = from
        self.title = title
        self.depth = depth
        self.chat = chat
    }

    /// Thread `id` as a linked exchange, when its row was asked by another.
    public static func of(_ id: String, rows: [ThreadRow], personas: [APIPersona]) -> Exchange? {
        guard let row = rows.first(where: { $0.id == id }), let from = rows.first(where: { $0.id == row.askedBy })
        else { return nil }
        let persona = { (id: String) in personas.first { $0.id == id } ?? APIPersona(id: id, name: id, agent: "") }
        var depth = 1
        var above = from.askedBy
        while let next = rows.first(where: { $0.id == above }) {
            depth += 1
            above = next.askedBy
        }
        let own = rows.first { $0.persona == row.persona && $0.askedBy.isEmpty && $0.members.isEmpty }
        return Exchange(
            asker: persona(from.persona), target: persona(row.persona), from: from.id, title: from.title,
            depth: depth, chat: own?.id
        )
    }
}

public enum LinkedItems {
    public static func build(_ items: [APIItem], exchange: Exchange, outbox: [Outgoing], now: Date) -> [Item] {
        var out: [Item] = []
        if let first = items.first {
            out.append(.divider(id: Int.min, text: "\(Clock.day(first.time, now: now)) · ask_persona"))
        }
        for item in items { place(item, answerer: exchange.target.id, into: &out) }
        return out + outbox.map(Item.outgoing)
    }

    private static func place(_ item: APIItem, answerer: String, into out: inout [Item]) {
        let time = Clock.time(item.time)
        let markdown = Markdown.blocks(item.text ?? "")
        switch item.kind {
        case "you" where !(item.speaker ?? "").isEmpty:
            out.append(.said(id: item.id, speaker: item.speaker ?? "", time: "asked · \(time)", markdown: markdown))
        case "agent":
            out.append(.said(id: item.id, speaker: answerer, time: "replied · \(time)", markdown: markdown))
        case "note":
            out.append(.divider(id: item.id, text: item.text ?? ""))
        case "tool":
            if case .tools = out.last {
            } else {
                out.append(.said(id: -item.id, speaker: answerer, time: time, markdown: []))
            }
            ChatItems.append(item, to: &out)
        default:
            ChatItems.append(item, to: &out)
        }
    }
}
