import SwiftUI

internal enum PanelTone: Sendable {
    case working
    case needs
    case done
    case idle

    internal func color(_ palette: Palette) -> Color {
        switch self {
        case .working: palette.cyan
        case .needs: palette.red
        case .done: palette.green
        case .idle: palette.mute
        }
    }
}

internal struct PanelIssue: Sendable {
    internal let number: Int
    internal let title: String
    internal let stage: String
    internal let tone: PanelTone
}

internal struct PanelRoutine: Sendable {
    internal let name: String
    internal let when: String
    internal let next: String
    internal let on: Bool
}

internal struct PanelPlace: Sendable {
    internal let name: String
    internal let symbol: String
    internal let count: Int
}

internal struct PanelContent: Sendable {
    internal var screen: [String] = []
    internal var issues: [PanelIssue] = []
    internal var moreIssues = 0
    internal var routines: [PanelRoutine] = []
    internal var places: [PanelPlace] = []
    internal var memory = 0
}

extension PanelContent {
    internal static let conductor = PanelContent(
        screen: [
            "$ herdsman board cooee",
            "#244 design   live  2m",
            "#236 queued   after #244",
            "#229 review1  PASS  ready",
            "#213 merged",
        ],
        issues: [
            PanelIssue(number: 244, title: "Persona panel design", stage: "design", tone: .working),
            PanelIssue(number: 229, title: "Workflow sheet", stage: "ready · merge?", tone: .needs),
            PanelIssue(number: 217, title: "Persona history tool", stage: "merged", tone: .done),
        ],
        moreIssues: 4,
        routines: [
            PanelRoutine(name: "Morning board", when: "Weekdays 08:30", next: "next tomorrow 08:30", on: true),
            PanelRoutine(name: "Stale PR sweep", when: "When CI goes red", next: "ran 2h ago", on: true),
            PanelRoutine(name: "Weekly lessons", when: "Fridays 17:00", next: "", on: false),
        ],
        places: [
            PanelPlace(name: "Queue", symbol: "list.number", count: 6),
            PanelPlace(name: "Events", symbol: "bolt.horizontal.fill", count: 12),
        ],
        memory: 9
    )

    internal static let coder = PanelContent(
        screen: [
            "$ npx playwright test look",
            "  ✓ 31 passed",
            "  ✗ 1 failed  look sheet › scene",
            "  running 32/50…",
        ],
        routines: [
            PanelRoutine(name: "Nightly e2e", when: "Every day 02:00", next: "next tonight 02:00", on: true)
        ],
        memory: 4
    )

    internal static func fixture(_ personaID: String) -> PanelContent {
        switch personaID {
        case "cond": conductor
        case "coder": coder
        default: PanelContent()
        }
    }
}
