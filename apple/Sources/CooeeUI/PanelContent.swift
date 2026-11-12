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
    internal let isOn: Bool
}

internal struct PanelPlace: Sendable {
    internal let name: String
    internal let symbol: String
    internal let count: Int
}

internal struct PanelContent: Sendable {
    internal var screen = ""
    internal var sites = 0
    internal var terminal: [String] = []
    internal var issues: [PanelIssue] = []
    internal var moreIssues = 0
    internal var routines: [PanelRoutine] = []
    internal var places: [PanelPlace] = []
    internal var memory = 0
}
