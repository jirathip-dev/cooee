import AppKit
import CooeeCore
@testable import CooeeUI
import Foundation
import SwiftUI
import Testing

@MainActor @Suite(.serialized)
internal struct IdleCPUTests {
    private static let seconds = Double(ProcessInfo.processInfo.environment["COOEE_CPU_SECONDS"] ?? "") ?? 10

    private static func used() -> Double {
        var usage = rusage()
        getrusage(RUSAGE_SELF, &usage)
        let seconds = { (time: timeval) in Double(time.tv_sec) + Double(time.tv_usec) / 1_000_000 }
        return seconds(usage.ru_utime) + seconds(usage.ru_stime)
    }

    private static var home: some View {
        var look = Look(palette: .farmDark)
        look.moving = true
        let personas = Fixtures.personas.map { $0.with(state: .idle) }
        let chats = Fixtures.rows.map { row in
            ListedChat(
                row: Row(
                    id: row.id, persona: row.persona, title: row.title, last: row.last, ago: row.ago,
                    unseen: row.unseen, state: .idle),
                persona: Fixtures.persona(row.persona).with(state: .idle))
        }
        return Sidebar(
            pins: personas.map(Pin.persona), all: personas, chats: chats, selected: "t5", actions: HomeActions()
        ) {
            EmptyView()
        } detail: {
            ChatView(personaID: "am", items: Fixtures.streaming)
        }
        .frame(width: 1280, height: 820).environment(\.look, look)
    }

    private static func idleShare() -> Double {
        let window = NSWindow(
            contentRect: CGRect(x: 0, y: 0, width: 1280, height: 820), styleMask: [.titled, .miniaturizable],
            backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: home)
        window.orderFrontRegardless()
        RunLoop.main.run(until: .now + 3)
        let (cpu, wall) = (used(), Date.now)
        RunLoop.main.run(until: .now + seconds)
        let share = (used() - cpu) / Date.now.timeIntervalSince(wall)
        window.close()
        FileHandle.standardError.write(Data("idle cpu \(String(format: "%.2f", share * 100))%\n".utf8))
        return share
    }

    @Test internal func anIdleHomeWithAChatOpenUsesUnderThreePercentCPU() {
        #expect(Self.idleShare() < 0.03)
    }

    private static func beats(_ beat: Beat, from start: Double, _ count: Int = 200) -> [Double] {
        beat.entries(from: Date(timeIntervalSinceReferenceDate: start), mode: .normal).prefix(count)
            .map(\.timeIntervalSinceReferenceDate)
    }

    @Test(arguments: [Beat(bar: 1.0 / 15), Beat(bar: 0.4), .idle])
    internal func figuresStartedApartRedrawOnTheSameMomentsAndTimeMovesOn(_ beat: Beat) {
        let one = Self.beats(beat, from: 812_345.0)
        let other = Self.beats(beat, from: 812_345.0 + 3 * beat.bar + 0.01)
        #expect(zip(one, one.dropFirst()).allSatisfy { $1 > $0 + 0.001 })
        #expect(other.dropFirst().prefix(100).allSatisfy { time in one.contains { abs($0 - time) < 0.000_001 } })
    }

    @Test internal func idleBirdsBlinkAndBreatheOnTheirSlowBeat() {
        let times = Self.beats(.idle, from: 812_345.0, 8)
        let stances = times.map { Stance(.idle, at: $0) }
        #expect(times.count == 8 && times[7] - times[0] < 8)
        #expect(stances.contains { $0.lid < 0.2 } && stances.contains { $0.lid == 1 })
        #expect(Set(stances.map(\.lift)).count > 2)
    }

    @Test internal func workingBirdsMoveAtFifteenFramesASecondAndDoneOnesHoldStill() throws {
        let beat = try #require(Beat.of(.working))
        let times = Self.beats(beat, from: 812_345.0, 16)
        #expect(times[15] - times[0] >= 1 - 0.001)
        #expect(Set(times.map { Stance(.working, at: $0).lean }).count == 16)
        #expect(Beat.of(.done) == nil && Beat.of(.fail) == nil)
    }

    @Test internal func aOneOffMotionStopsOnItsRestPose() throws {
        let start = Date(timeIntervalSinceReferenceDate: 812_345.0)
        let ends = try #require(Motion.hop.ends)
        let beat = Beat(bar: try #require(Motion.hop.every), until: start + ends)
        let times = Array(beat.entries(from: start, mode: .normal).prefix(1_000))
        #expect(times.count < 1_000 && times.last == start + ends)
        #expect(Motion.hop.pose(at: try #require(times.last).timeIntervalSince(start)) == .rest)
        #expect(Motion.still.every == nil)
    }
}
