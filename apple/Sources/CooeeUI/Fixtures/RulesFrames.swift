import CooeeCore
import SwiftUI

/// The rules design's frames (#536, apple/design/rules), drawn from the views the app shows.
public enum RulesFrames {
    @MainActor
    public static func list() -> some View { RulesStory.page() }

    @MainActor
    public static func add() -> some View {
        ZStack(alignment: .bottom) {
            RulesStory.page()
            Dim()
            AddRuleSheet(draft: .constant(RulesStory.draft), folder: RulesStory.folder)
        }
    }

    @MainActor
    public static func remove() -> some View {
        ZStack(alignment: .bottom) {
            RulesStory.page(swiped: "3f")
            Dim()
            RemoveSheet(rule: RulesStory.coder[2])
        }
    }

    @MainActor
    public static func picker() -> some View {
        ZStack(alignment: .bottom) {
            RulesPersonaSheet().background(Backdrop())
            Dim()
            AgentSheet(draft: .constant(RulesStory.persona), choices: RulesStory.choices)
        }
    }

    @MainActor
    public static func mac() -> some View { MacRules() }
}

internal enum RulesStory {
    internal static let folder = "~/src/cooee"

    internal static let coder: [Rule] = [
        rule("a1", .deny, .tool, "Deny using github/delete_repo", ("settings", 1_794_484_800_000)),
        rule("07", .ask, .run, "Ask before running git push … in ~/src/cooee", ("settings", 1_794_657_600_000)),
        rule("3f", .allow, .run, "Allow running cargo test … in ~/src/cooee", ("card", 1_795_003_200_000)),
        rule("5c", .allow, .fetch, "Allow fetching from docs.rs", ("card", 1_795_089_600_000)),
        rule("9e", .allow, .edit, "Allow editing notes/** in ~/src/cooee", ("settings", 1_795_176_000_000)),
        rule("01", .allow, .read, "Allow reading any file in ~/src/cooee", ("starter", 0)),
        rule("02", .allow, .search, "Allow searching any file in ~/src/cooee", ("starter", 0)),
    ]

    internal static let draft = NewRule(effect: .allow, kind: .run, pattern: "npm run lint *", folder: folder)

    private static let personal = Gap(words: "Reads your own instructions and settings on this sign-in")

    internal static let agents: [AgentOption] = [
        AgentOption(id: "Claude Code"),
        AgentOption(
            id: "Codex", efforts: ["low", "medium", "high"],
            gaps: [
                personal, Gap(kind: .read, words: "Never asks before reading a file in its sandbox"),
                Gap(kind: .search, words: "Never asks before a search in its sandbox"),
                Gap(kind: .run, words: "Never asks before a command in its sandbox"),
            ]),
        AgentOption(id: "Gemini", gaps: [personal]),
    ]

    internal static let persona: APIPersona = {
        var coder = APIPersona(id: "coder", name: "Coder", agent: "Codex", model: "gpt-5.5", effort: "medium")
        coder.instructions = "You work on cooee. Run the tests before you say it's done."
        coder.folder = folder
        return coder
    }()

    internal static let choices: PersonaChoices = {
        var choices = PersonaChoices(agents: agents, models: ["gpt-5.5", "gpt-5.5-mini"], efforts: ["low", "medium"])
        choices.lists = [
            ModelList(agent: "Claude Code", models: [["haiku 5.5"], ["opus 5.5"], ["sonnet 5.5"]]),
            ModelList(agent: "Codex", models: [["gpt-5.5"], ["gpt-5.5-mini"]]),
            ModelList(agent: "Gemini", models: [["gemini 3 flash"], ["gemini 3 pro"]]),
        ]
        choices.rules = coder.count
        return choices
    }()

    @MainActor
    internal static func page(swiped: String? = nil) -> some View {
        RulesPage(name: "Coder", rules: Rule.strictestFirst(coder), agent: agents[1], swiped: swiped)
    }

    private static func rule(
        _ id: String, _ effect: RuleEffect, _ kind: ToolKind, _ words: String, _ from: (String, Double)
    ) -> Rule {
        Rule(id: id, effect: effect, kind: kind, pattern: "", source: from.0, made: from.1, words: words)
    }
}

internal struct RulesPersonaSheet: View {
    @Environment(\.look) private var look
    internal var rulesPicked = false

    internal var body: some View {
        let palette = look.palette
        VStack(spacing: 14) {
            HStack {
                RoundButton(symbol: "xmark", ascii: "x")
                Spacer()
                Text("Coder").ui(17, .semibold, look)
                Spacer()
                RoundButton(symbol: "checkmark", ascii: "ok")
            }
            .padding(.horizontal, 16).padding(.top, 14)
            VStack(alignment: .leading, spacing: 18) {
                Group2(header: "Instructions") {
                    Line(divider: false) { Text(RulesStory.persona.instructions).ui(16, .regular, look) }
                }
                Group2(header: "Agent") {
                    PersonaAgentRows(
                        draft: .constant(RulesStory.persona), choices: RulesStory.choices, pick: { _ in }, rules: {},
                        rulesPicked: rulesPicked
                    )
                }
                Group2 {
                    SettingRow(symbol: "folder.fill", title: "Folder", tint: palette.amber, value: RulesStory.folder)
                    SettingRow(
                        symbol: "desktopcomputer", title: "Machine", tint: palette.cyan, value: "studio", divider: false
                    )
                }
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 16)
        }
        .background(look.outline ? palette.background : palette.card.opacity(0.35))
        .foregroundStyle(palette.foreground)
    }
}

internal struct MacRules: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 0) {
            RulesPersonaSheet(rulesPicked: true).padding(.top, 34).frame(width: 440)
                .overlay(alignment: .trailing) { Rectangle().fill(palette.line).frame(width: 1) }
            VStack(alignment: .leading, spacing: 0) {
                Text(look.label(.section, "Coder's rules")).ui(26, .bold, look)
                    .padding(.horizontal, 28).padding(.top, 48).padding(.bottom, 4)
                Scroller {
                    RulesBody(
                        name: "Coder", rules: Rule.strictestFirst(RulesStory.coder), agent: RulesStory.agents[1],
                        hover: "5c"
                    )
                    .frame(maxWidth: 720, alignment: .leading).padding(.horizontal, 28).padding(.vertical, 18)
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .foregroundStyle(palette.foreground)
        .background(Backdrop())
    }
}
