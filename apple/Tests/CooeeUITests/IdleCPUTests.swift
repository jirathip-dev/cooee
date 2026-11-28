import AppKit
import CooeeCore
@testable import CooeeUI
import Darwin
import Foundation
import SwiftUI
import Testing

@MainActor @Suite(.serialized)
internal struct IdleCPUTests {
    private static let seconds = Double(ProcessInfo.processInfo.environment["COOEE_CPU_SECONDS"] ?? "") ?? 5

    private static func used() -> Double {
        var usage = rusage()
        getrusage(RUSAGE_SELF, &usage)
        let seconds = { (time: timeval) in Double(time.tv_sec) + Double(time.tv_usec) / 1_000_000 }
        return seconds(usage.ru_utime) + seconds(usage.ru_stime)
    }

    private static func home(_ state: LaneState) -> some View {
        var look = Look(palette: .farmDark)
        look.moving = true
        let personas = Fixtures.personas.map { $0.with(state: state) }
        let chats = Fixtures.rows.map { row in
            ListedChat(
                row: Row(
                    id: row.id, persona: row.persona, title: row.title, last: row.last, ago: row.ago,
                    unseen: row.unseen, state: state),
                persona: Fixtures.persona(row.persona).with(state: state))
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

    private static func share(_ state: LaneState, shown: (NSWindow) -> Void = { $0.orderFrontRegardless() }) -> Double {
        let window = NSWindow(
            contentRect: CGRect(x: 0, y: 0, width: 1280, height: 820), styleMask: [.titled, .miniaturizable],
            backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: home(state))
        shown(window)
        RunLoop.main.run(until: .now + 1)
        let (cpu, wall) = (used(), Date.now)
        RunLoop.main.run(until: .now + seconds)
        let share = (used() - cpu) / Date.now.timeIntervalSince(wall)
        window.close()
        FileHandle.standardError.write(Data("cpu \(state) \(String(format: "%.2f", share * 100))%\n".utf8))
        return share
    }

    @Test internal func anIdleHomeWithAChatOpenUsesUnderThreePercentCPU() {
        #expect(Self.share(.idle) < 0.03)
    }
}
