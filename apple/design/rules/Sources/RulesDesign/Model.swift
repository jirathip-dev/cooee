import CooeeCore
@testable import CooeeUI
import SwiftUI

internal enum Effect: String, Sendable, CaseIterable {
    case allow, ask, deny

    internal var title: String {
        switch self {
        case .allow: "Allow"
        case .ask: "Ask first"
        case .deny: "Deny"
        }
    }

    internal func tint(_ palette: Palette) -> Color {
        switch self {
        case .allow: palette.green
        case .ask: palette.amber
        case .deny: palette.red
        }
    }
}

internal struct RuleFixture: Sendable, Identifiable {
    internal let id: String
    internal let effect: Effect
    internal let kind: ToolKind
    internal let words: String
    internal let from: String
    internal var silent = false
}

internal struct AgentFixture: Sendable, Identifiable {
    internal let id: String
    internal let models: String
    internal let gaps: [String]
}

internal enum Story {
    internal static let coder: [RuleFixture] = [
        RuleFixture(id: "a1", effect: .deny, kind: .tool, words: "Deny using github/delete_repo", from: "added here · 12 Nov"),
        RuleFixture(
            id: "07", effect: .ask, kind: .run, words: "Ask before running git push … in ~/src/cooee",
            from: "added here · 14 Nov"),
        RuleFixture(
            id: "3f", effect: .allow, kind: .run, words: "Allow running cargo test … in ~/src/cooee",
            from: "from a card · 18 Nov", silent: true),
        RuleFixture(id: "5c", effect: .allow, kind: .fetch, words: "Allow fetching from docs.rs", from: "from a card · 19 Nov"),
        RuleFixture(
            id: "9e", effect: .allow, kind: .edit, words: "Allow editing notes/** in ~/src/cooee",
            from: "added here · 20 Nov"),
        RuleFixture(
            id: "01", effect: .allow, kind: .read, words: "Allow reading any file in ~/src/cooee", from: "starter rule",
            silent: true),
        RuleFixture(
            id: "02", effect: .allow, kind: .search, words: "Allow searching any file in ~/src/cooee",
            from: "starter rule", silent: true),
    ]

    internal static let personal = "Reads your own instructions and settings on this sign-in"

    internal static let agents: [AgentFixture] = [
        AgentFixture(id: "Claude Code", models: "opus 5.5 · sonnet 5.5 · haiku 5.5", gaps: []),
        AgentFixture(
            id: "Codex", models: "gpt-5.5 · gpt-5.5-mini",
            gaps: [
                personal, "Never asks before reading a file in its sandbox", "Never asks before a search in its sandbox",
                "Never asks before a command in its sandbox",
            ]),
        AgentFixture(id: "Gemini", models: "gemini 3 pro · flash", gaps: [personal]),
    ]
}
