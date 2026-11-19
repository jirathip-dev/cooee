import Foundation

/// A voice call kept in its thread: its spoken lines, how long it ran and how many tools it used.
public struct Call: Sendable, Equatable {
    public struct Line: Sendable, Equatable {
        public let you: Bool
        public let text: String

        public init(you: Bool, text: String) {
            self.you = you
            self.text = text
        }
    }

    public struct Part: Sendable, Equatable {
        public var call: Call?
        public var items: [APIItem] = []
        public var last = false
    }

    public let id: Int
    public var seconds: Int
    public var lines: [Line]
    public var tools: Int

    public init(id: Int, seconds: Int, lines: [Line], tools: Int) {
        self.id = id
        self.seconds = seconds
        self.lines = lines
        self.tools = tools
    }

    /// The thread cut at its calls: each run of `via: "voice"` turns is one call, with its tool steps after it.
    public static func parts(_ items: [APIItem]) -> [Part] {
        var parts: [Part] = []
        var start: Int64 = 0
        for item in items {
            let calling = parts.last?.call != nil
            if item.kind == "you", item.voice, !calling {
                parts.append(Part(call: Call(id: item.id, seconds: 0, lines: [], tools: 0)))
                start = item.time
            } else if parts.isEmpty || (calling && item.kind == "you" && !item.voice) {
                parts.append(Part())
            }
            parts[parts.count - 1].take(item, since: start)
        }
        if !parts.isEmpty { parts[parts.count - 1].last = true }
        return parts
    }

    fileprivate mutating func take(_ item: APIItem, since start: Int64) {
        seconds = max(seconds, Int(item.time - start) / 1000)
        switch item.kind {
        case "you", "agent": lines.append(Line(you: item.kind == "you", text: item.text ?? ""))
        case "tool": tools += 1
        default: break
        }
    }
}

extension Call.Part {
    fileprivate mutating func take(_ item: APIItem, since start: Int64) {
        guard var call else { return items.append(item) }
        call.take(item, since: start)
        self.call = call
        if item.kind != "you" && item.kind != "agent" { items.append(item) }
    }
}
