import CooeeCore
import CooeeFeatures
import CooeeUI
import Foundation
import SwiftUI
import Testing

@Suite(.serialized)
internal struct MenuBarFrames {
    private static let pair = URL(string: "https://mini.tail2b4.ts.net/pair")

    private static func chat(
        _ persona: String, _ state: LaneState, title: String, last: String = "", ago: String,
        lane: (String, String)? = nil
    ) -> ListedChat {
        let workflow = lane.map { WorkflowLabel(source: "drover", group: "cooee", item: $0.0, role: $0.1, round: "1") }
        let row = Row(
            id: "\(persona)-\(title)", persona: persona, title: title, last: last, ago: ago, unseen: 0, state: state)
        return ListedChat(row: row, persona: Fixtures.persona(persona).with(state: state), workflow: workflow)
    }

    private static let asks = [
        chat("res", .needs, title: "Filings", last: "Allow fetch on sec.gov?", ago: "12m"),
        chat("cond", .needs, title: "design", last: "Approve the menu bar?", ago: "4m", lane: ("#659", "design")),
        chat("am", .needs, title: "impl", last: "blocked: needs a secret", ago: "1h", lane: ("#618", "impl lane")),
    ]

    private static let running = [
        chat("coder", .working, title: "Export for Kestrel", ago: "2m"),
        chat("chief", .working, title: "Q3 board deck", ago: "now"),
        chat("cond", .working, title: "rev lane", ago: "5m", lane: ("#631", "review")),
    ]

    private static let clocks = Dictionary(
        uniqueKeysWithValues: zip(running.map(\.id), [134.0, 48, 302].map(Glance.clock)))

    internal static let states: [String: (CGFloat, Glance)] = [
        "running": (674, Glance(link: .running("mini"), pair: pair, asks: asks, running: running, clocks: clocks)),
        "quiet": (462, Glance(link: .running("mini"), pair: pair)),
        "unreachable": (
            554,
            Glance(
                link: .unreachable("mini", since: "9:12"), pair: pair, asks: Array(asks.prefix(1)),
                running: Array(running.prefix(1)), clocks: clocks)
        ),
        "unpaired": (379, Glance(link: .unpaired)),
    ]

    @MainActor @Test(arguments: states.keys.sorted(), ["dark", "light"])
    internal func matchesTheDesign(_ state: String, _ mode: String) throws {
        let (height, glance) = try #require(Self.states[state])
        let palette: Palette = mode == "dark" ? .farmDark : .farmLight
        let panel = MenuPanel(glance).padding(25).frame(width: 390, height: height).background(palette.background)
            .environment(\.look, Look(palette: palette, style: .bundled, still: true))
            .environment(\.colorScheme, palette.dark ? .dark : .light)
        let renderer = ImageRenderer(content: panel)
        renderer.scale = 2
        try Frame.matches(try #require(renderer.cgImage), "\(state)-390-\(mode)", in: "menu-bar")
    }
}
