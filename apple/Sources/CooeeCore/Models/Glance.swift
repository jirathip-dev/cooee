import Foundation

/// One row of `GET /glance`: a thread or a lane that needs you or runs.
public struct GlanceRow: Codable, Sendable, Equatable, Identifiable {
    public var thread: String
    public var title: String
    public var persona: String
    public var name: String
    public var machine: String
    public var line: String
    public var since: Int
    public var lane: Bool

    public var id: String { thread }

    public init(
        thread: String, title: String, persona: String, name: String, machine: String = "", line: String,
        since: Int = 0, lane: Bool = false
    ) {
        self.thread = thread
        self.title = title
        self.persona = persona
        self.name = name
        self.machine = machine
        self.line = line
        self.since = since
        self.lane = lane
    }
}

/// What the widgets show: the asks, the blocked lanes and what runs, as of `time` (Unix milliseconds).
public struct Glance: Codable, Sendable, Equatable {
    private enum CodingKeys: String, CodingKey {
        case time = "at"
        case asks, blocked, running
    }

    public var time: Int
    public var asks: [GlanceRow]
    public var blocked: [GlanceRow]
    public var running: [GlanceRow]

    public var needs: [GlanceRow] { asks + blocked }

    public init(time: Int = 0, asks: [GlanceRow] = [], blocked: [GlanceRow] = [], running: [GlanceRow] = []) {
        self.time = time
        self.asks = asks
        self.blocked = blocked
        self.running = running
    }
}

/// The one Live Activity's state, as the hub pushes it: the top thread, then the rest as `+N`.
public struct Shown: Codable, Sendable, Hashable {
    public var kind: String
    public var thread: String
    public var title: String
    public var persona: String
    public var name: String
    public var machine: String
    public var line: String
    public var since: Int
    public var more: String
    public var tools: Int

    public init(
        kind: String, thread: String, title: String, persona: String, name: String, machine: String = "",
        line: String, since: Int = 0, more: String = "", tools: Int = 0
    ) {
        self.kind = kind
        self.thread = thread
        self.title = title
        self.persona = persona
        self.name = name
        self.machine = machine
        self.line = line
        self.since = since
        self.more = more
        self.tools = tools
    }
}

/// The App Group copy the app writes and the widgets and intents read: the glance, the personas, the recent threads,
/// the server's host and the style drawn (none: Modern).
public struct GlanceCache: Codable, Sendable, Equatable {
    public static let group = "group.com.jirathip.cooee"

    public var glance: Glance
    public var personas: [APIPersona]
    public var threads: [GlanceRow]
    public var host: String
    public var style: Style?

    public init(glance: Glance, personas: [APIPersona], threads: [GlanceRow] = [], host: String, style: Style? = nil) {
        self.glance = glance
        self.personas = personas
        self.threads = threads
        self.host = host
        self.style = style
    }

    /// `persona`'s latest thread.
    public func latest(of persona: String) -> GlanceRow? {
        threads.filter { $0.persona == persona }.max { $0.since < $1.since }
    }

    public static func file(in folder: URL?) -> URL? { folder?.appendingPathComponent("glance.json") }

    public static func read(_ file: URL?) -> GlanceCache? {
        file.flatMap { try? Data(contentsOf: $0) }.flatMap { try? JSONDecoder().decode(Self.self, from: $0) }
    }

    public func write(_ file: URL?) throws {
        guard let file else { return }
        try JSONEncoder().encode(self).write(to: file, options: .atomic)
    }

    /// The age line: `as of 9:41 · mini`.
    public func asOf(_ calendar: Calendar = .current) -> String {
        let time = Date(timeIntervalSince1970: Double(glance.time) / 1000)
        let parts = calendar.dateComponents([.hour, .minute], from: time)
        let clock = String(format: "%d:%02d", parts.hour ?? 0, parts.minute ?? 0)
        return host.isEmpty ? "as of \(clock)" : "as of \(clock) · \(host)"
    }
}

/// A clock between two Unix-millisecond times: `2:14` for a running turn, `12m` for a wait.
public enum Since {
    public static func clock(_ since: Int, _ now: Int) -> String {
        let seconds = max(0, now - since) / 1000
        return String(format: "%d:%02d", seconds / 60, seconds % 60)
    }

    public static func wait(_ since: Int, _ now: Int) -> String {
        let minutes = max(0, now - since) / 60_000
        return minutes < 60 ? "\(minutes)m" : "\(minutes / 60)h"
    }
}
