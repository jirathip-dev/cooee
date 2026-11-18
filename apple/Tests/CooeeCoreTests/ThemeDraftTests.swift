@testable import CooeeCore
import Foundation
import Testing

internal struct ThemeDraftTests {
    @Test internal func aLabelNamesItsFolder() {
        #expect(ThemeDraft.folder("Rock Pool 2!") == "rock-pool-2")
        #expect(ThemeDraft.folder("  Tidepool  ") == "tidepool")
        #expect(ThemeDraft.folder("Café") == "caf")
    }

    @Test(arguments: ["#a88a2a", "#c0203f", "#1f7a45", "#13262d", "#ffffff", "#000000", "#7a4aa8", "#1c6e70"])
    internal func aShadeKeepsItsColour(_ hex: String) throws {
        let shade = try #require(Shade(hex: hex))
        #expect(shade.hex == hex)
    }

    @Test internal func aShadeMovesByHueAndLight() throws {
        var shade = try #require(Shade(hex: "#ff0000"))
        #expect(shade.hue == 0 && shade.light == 0.5 && shade.saturation == 1)
        shade.hue = 1.0 / 3
        #expect(shade.hex == "#00ff00")
        shade.light = 0.25
        #expect(shade.hex == "#008000")
        #expect(Shade(hex: "#0000ff")?.hue == 2.0 / 3 && Shade(hex: "#ff00ff")?.hue == 5.0 / 6)
        #expect(Shade(hex: "red") == nil && Shade(hex: "#zzzzzz") == nil)
    }

    @Test internal func aMarkPassesAtItsMinimum() throws {
        let json = #"{"what": "--a on --card", "value": 3, "min": 3}"#
        let mark = try JSONDecoder().decode(GateMark.self, from: Data(json.utf8))
        #expect(mark.passes && mark.fix == nil)
        let report = try JSONEncoder().encode(GateReport(fails: ["x"], marks: ["--a": mark]))
        #expect(try JSONDecoder().decode(GateReport.self, from: report).marks["--a"] == mark)
    }
}
