import CooeeCore
import SwiftUI

extension SurfaceGlance {
    /// The widgets' rows from the App Group copy, with clocks at `now` (Unix milliseconds).
    public init(_ cache: GlanceCache, now: Int) {
        let glance = cache.glance
        let who = { (row: GlanceRow, state: LaneState) in
            Persona.of(row.persona, row.name, state, in: cache.personas)
        }
        let asks = glance.asks.map {
            SurfaceRow(
                thread: $0.thread, persona: who($0, .needs), title: "\($0.name) asks", detail: $0.line, trailing: "ask")
        }
        let blocked = glance.blocked.map {
            SurfaceRow(
                thread: $0.thread, persona: who($0, .fail), title: $0.title, detail: "blocked: \($0.line)",
                trailing: "blocked")
        }
        let running = glance.running.map {
            SurfaceRow(
                thread: $0.thread, persona: who($0, .working), title: $0.lane ? $0.title : $0.name,
                detail: $0.lane ? $0.line : $0.title, trailing: Since.clock($0.since, now))
        }
        self.init(
            needs: asks + blocked, running: running,
            counts: Self.counted([(glance.asks.count, ("ask", "asks")), (glance.blocked.count, ("blocked", "blocked"))]
            ),
            rest: Self.rest(Array(glance.running.dropFirst(2))), asOf: cache.asOf())
    }

    private static func counted(_ parts: [(count: Int, words: (one: String, many: String))]) -> String {
        let shown = parts.filter { $0.count > 0 }
        return shown.map { "\($0.count) \($0.count == 1 ? $0.words.one : $0.words.many)" }.joined(separator: " · ")
    }

    private static func rest(_ rows: [GlanceRow]) -> String {
        let lanes = rows.count { $0.lane }
        let parts = counted([(rows.count - lanes, ("turn", "turns")), (lanes, ("lane", "lanes"))])
        return parts.isEmpty ? "" : "+" + parts.replacingOccurrences(of: " · ", with: " · +")
    }
}

extension SurfaceActivity {
    /// The Live Activity from the hub's pushed state at `now` (Unix milliseconds).
    public init(_ shown: Shown, personas: [APIPersona], now: Int) {
        let state: LaneState =
            switch shown.kind {
            case "ask": .needs
            case "failed": .fail
            case "running": .working
            default: .done
            }
        let place = shown.machine.isEmpty ? shown.title : "\(shown.title) · \(shown.machine)"
        let (headline, foot): (String, String) =
            switch state {
            case .needs: ("\(shown.name) asks you", "Tap to open the ask")
            case .fail: ("\(shown.name) failed", "Tap to open")
            case .working: (shown.name, "working · \(shown.tools) tools")
            default: (shown.name, "replied")
            }
        self.init(
            thread: shown.thread, persona: Persona.of(shown.persona, shown.name, state, in: personas),
            headline: headline, place: place,
            clock: state == .working ? Since.clock(shown.since, now) : Since.wait(shown.since, now), line: shown.line,
            foot: foot, more: shown.more, tools: shown.tools)
        start = Date(timeIntervalSince1970: Double(shown.since) / 1000)
    }
}
