import CooeeCore
@testable import CooeeUI
import Foundation
import Testing

internal struct SurfaceMappingTests {
    private static let now = 1_000_000
    private let researcher = APIPersona(id: "res", name: "Researcher", agent: "x")

    private func row(_ id: String, _ name: String, _ title: String, _ line: String, lane: Bool = false) -> GlanceRow {
        GlanceRow(
            thread: id, title: title, persona: String(name.lowercased().prefix(3)), name: name, line: line,
            since: Self.now - 134_000, lane: lane)
    }

    @Test internal func theWidgetsRowsReadTheGlanceAsDrawn() {
        let glance = Snapshot(
            time: 0, asks: [row("a", "Researcher", "Kestrel", "Allow fetch on sec.gov?")],
            blocked: [row("b", "Am", "cooee#218 · impl lane", "needs a secret", lane: true)],
            running: [
                row("r1", "Coder", "Export for Kestrel", "$ cargo"), row("r2", "Cond", "cooee queue", "…"),
                row("r3", "Chief", "cooee#231 · review", "rev lane", lane: true),
            ])
        let surface = SurfaceGlance(GlanceCache(glance: glance, personas: [researcher], host: "mini"), now: Self.now)
        let seen = (surface.needs + surface.running).map { [$0.title, $0.detail, $0.trailing] }
        #expect(
            seen == [
                ["Researcher asks", "Allow fetch on sec.gov?", "ask"],
                ["cooee#218 · impl lane", "blocked: needs a secret", "blocked"],
                ["Coder", "Export for Kestrel", "2:14"], ["Cond", "cooee queue", "2:14"],
                ["cooee#231 · review", "rev lane", "2:14"],
            ])
        #expect(
            surface.needs.map(\.persona.state) == [.needs, .fail]
                && surface.top?.url.absoluteString == "cooee://thread/a")
        #expect(surface.needs[0].persona.api == researcher && surface.running[0].persona.name == "Coder")
        #expect((surface.counts, surface.rest) == ("1 ask · 1 blocked", "+1 lane"))
        let two = SurfaceGlance(
            GlanceCache(glance: Snapshot(asks: glance.asks + glance.asks), personas: [], host: ""), now: Self.now)
        #expect((two.counts, two.rest) == ("2 asks", ""))
    }

    @Test internal func theLiveActivityReadsTheHubsStateForEachKind() {
        let shown = Shown(
            kind: "ask", thread: "t2", title: "Kestrel filings", persona: "res", name: "Researcher", machine: "mini",
            line: "Allow fetch?", since: Self.now - 12 * 60_000, more: "+2 running", tools: 6)
        let card = { (kind: String) in
            var state = shown
            state.kind = kind
            let activity = SurfaceActivity(state, personas: [researcher], now: Self.now)
            return [activity.headline, activity.place, activity.clock, activity.foot, activity.more]
        }
        #expect(
            card("ask") == [
                "Researcher asks you", "Kestrel filings · mini", "12m", "Tap to open the ask", "+2 running",
            ])
        #expect(
            card("running") == ["Researcher", "Kestrel filings · mini", "12:00", "working · 6 tools", "+2 running"])
        #expect(card("failed") == ["Researcher failed", "Kestrel filings · mini", "12m", "Tap to open", "+2 running"])
        #expect(card("ended") == ["Researcher", "Kestrel filings · mini", "12m", "replied", "+2 running"])
        let activity = SurfaceActivity(shown, personas: [], now: Self.now)
        #expect(activity.asks && !activity.running && activity.persona.name == "Researcher")
        #expect(activity.start == Date(timeIntervalSince1970: Double(Self.now - 12 * 60_000) / 1000))
    }
}
