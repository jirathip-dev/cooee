import CooeeCore
import Testing

internal struct TimingsTests {
    @Test internal func p50AndP95AreOverTheLastHundredRunsOnly() {
        var timings = Timings()
        #expect(timings.isEmpty && timings.percentile(.tap, 95) == nil)
        for run in 1...100 { timings.record(.tap, millis: 1000 + run) }
        for run in 1...100 { timings.record(.tap, millis: run) }
        timings.record(.event, millis: Timings.millis(.milliseconds(140) + .microseconds(900)))
        #expect(timings.percentile(.tap, 50) == 50 && timings.percentile(.tap, 95) == 95)
        #expect(timings.percentile(.event, 95) == 140 && timings.percentile(.echo, 50) == nil)
    }
}
