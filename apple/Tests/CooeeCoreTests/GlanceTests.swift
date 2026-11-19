import CooeeCore
import Foundation
import Testing

internal struct GlanceTests {
    private let ask = GlanceRow(thread: "a", title: "Kestrel", persona: "res", name: "Researcher", line: "Allow?")

    @Test internal func theAppGroupCopyReadsBackWhatTheAppWroteAndNothingWithoutAFolder() throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("glance-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let file = GlanceCache.file(in: folder)
        #expect(GlanceCache.read(file) == nil)
        let cache = GlanceCache(
            glance: Glance(at: 9, asks: [ask]), personas: [APIPersona(id: "res", name: "Researcher", agent: "x")],
            host: "mini")
        try cache.write(file)
        #expect(GlanceCache.read(file) == cache)
        #expect(GlanceCache.file(in: nil) == nil && GlanceCache.read(nil) == nil)
        try cache.write(nil)
    }

    @Test internal func itsAgeLineNamesTheTimeAndTheServer() throws {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = try #require(TimeZone(identifier: "UTC"))
        let at = (9 * 60 + 41) * 60_000
        let cache = GlanceCache(glance: Glance(at: at), personas: [], host: "mini")
        #expect(cache.asOf(calendar) == "as of 9:41 · mini")
        var bare = cache
        bare.host = ""
        #expect(bare.asOf(calendar) == "as of 9:41")
    }

    @Test internal func thePersonasLatestThreadIsItsNewest() {
        let old = GlanceRow(thread: "o", title: "Old", persona: "coder", name: "Coder", line: "", since: 1)
        let new = GlanceRow(thread: "n", title: "New", persona: "coder", name: "Coder", line: "", since: 5)
        let cache = GlanceCache(glance: Glance(), personas: [], threads: [old, new, ask], host: "")
        #expect(cache.latest(of: "coder") == new && cache.latest(of: "chief") == nil)
    }

    @Test internal func clocksCountUpAndWaitsCountMinutesThenHours() {
        #expect(Since.clock(1_000, 135_000) == "2:14" && Since.clock(5_000, 0) == "0:00")
        #expect(Since.wait(0, 12 * 60_000) == "12m" && Since.wait(0, 90 * 60_000) == "1h")
    }

    @Test internal func whatNeedsYouIsTheAsksThenTheBlockedLanesAndTheHubsStateDecodes() throws {
        let lane = GlanceRow(thread: "l", title: "cooee#218", persona: "", name: "", line: "needs a secret", lane: true)
        #expect(Glance(asks: [ask], blocked: [lane]).needs == [ask, lane])
        let pushed = #"{"kind":"running","thread":"t","title":"Export","persona":"coder","name":"Coder","#
            + #""machine":"studio","line":"wrote 412 rows","since":7,"more":"+2 running","tools":6}"#
        let shown = try JSONDecoder().decode(Shown.self, from: Data(pushed.utf8))
        #expect(
            shown
                == Shown(
                    kind: "running", thread: "t", title: "Export", persona: "coder", name: "Coder", machine: "studio",
                    line: "wrote 412 rows", since: 7, more: "+2 running", tools: 6))
    }
}
