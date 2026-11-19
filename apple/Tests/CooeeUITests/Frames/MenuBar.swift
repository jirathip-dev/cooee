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

    internal static let states: [String: (CGFloat, Glance)] = [
        "running": (674, Glance(link: .running("mini"), pair: pair, asks: asks, running: running)),
        "quiet": (462, Glance(link: .running("mini"), pair: pair)),
        "unreachable": (
            554,
            Glance(
                link: .unreachable("mini", since: "9:12"), pair: pair, asks: Array(asks.prefix(1)),
                running: Array(running.prefix(1)))
        ),
        "unpaired": (379, Glance(link: .unpaired)),
    ]

    internal static var frames: [Frame] {
        states.flatMap { name, panel in
            [("dark", Palette.farmDark), ("light", Palette.farmLight)].map { mode, palette in
                Frame.mac("\(name)-390-\(mode)", 390, panel.0, palette) { MenuPanel(panel.1).padding(25) }
                    .bare().filed("menu-bar")
            }
        }
    }

    @MainActor @Test(arguments: frames)
    internal func matchesTheDesign(_ frame: Frame) throws { try frame.matchesItsDesign() }
}
