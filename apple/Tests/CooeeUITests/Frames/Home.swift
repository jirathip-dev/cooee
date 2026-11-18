import CooeeCore
import CooeeFeatures
import CooeeUI
import SwiftUI
import Testing

@Suite(.serialized)
internal struct HomeFrames {
    internal static var all: [Frame] {
        [("dark", Palette.farmDark), ("light", Palette.farmLight)].flatMap { phones($0.0, $0.1) } + macs
    }

    private static func phones(_ mode: String, _ palette: Palette) -> [Frame] {
        [
            .phone("home-\(mode)", palette) { HomeView() },
            .phone("chat-streaming-\(mode)", palette) { ChatView.streaming() },
            .phone("chat-card-\(mode)", palette) { ChatView.cards() },
            .phone("chat-cards-more-\(mode)", palette) { ChatView.moreCards() },
            .phone("chat-conductor-\(mode)", palette) { ChatView.conductor() },
            .phone("persona-\(mode)", palette) { PersonaEditBlocks() },
            .phone("avatar-menu-\(mode)", palette) { HomeView(menuOpen: true) },
            .phone("persona-panel-\(mode)", palette) { PersonaPanelBlocks() },
        ]
            + PhoneStep.allCases.map { step in
                .phone("pair-\(step.rawValue)-\(mode)", palette) { PairingBlocks(step: step) }
            }
    }

    private static var macs: [Frame] {
        [
            .mac("mac-connect", 960, 640, .farmLight) { MacConnect() },
            .mac("mac-window-dark", 1280, 820, .farmDark) { Sidebar() },
            .mac("mac-window-light", 1280, 820, .farmLight) { Sidebar() },
            .mac("mac-persona-panel-dark", 360, 820, .farmDark) {
                PersonaPanelBlocks(personaID: "coder", column: true)
            },
        ]
    }

    private static var panels: [Frame] {
        [
            .phone("persona-panel-end-dark", .farmDark) { PersonaPanelBlocks(end: true) },
            .phone("persona-panel-empty-dark", .farmDark) { PersonaPanelBlocks(personaID: "chief") },
        ]
    }

    @MainActor @Test(arguments: all + panels)
    internal func matchesTheDesign(_ frame: Frame) throws { try frame.matchesItsDesign() }

    @MainActor @Test(arguments: all)
    internal func fitsInEveryPack(_ frame: Frame) throws { try frame.fitsInEveryPack() }
}
