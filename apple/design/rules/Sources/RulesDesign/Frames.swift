import CooeeCore
@testable import CooeeUI
import SwiftUI

public enum RulesFrames {
    @MainActor
    public static func list() -> some View { RulesPage() }

    @MainActor
    public static func add() -> some View {
        ZStack(alignment: .bottom) {
            RulesPage()
            Dim()
            AddRuleSheet()
        }
    }

    @MainActor
    public static func remove() -> some View {
        ZStack(alignment: .bottom) {
            RulesPage(swiped: "3f")
            Dim()
            RemoveSheet(rule: Story.chief[2])
        }
    }

    @MainActor
    public static func picker() -> some View {
        ZStack(alignment: .bottom) {
            PersonaSheet().background(Backdrop())
            Dim()
            AgentSheet()
        }
    }

    @MainActor
    public static func mac() -> some View { MacRules() }
}

internal struct MacRules: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 0) {
            PersonaSheet(rulesPicked: true).padding(.top, 34).frame(width: 440)
                .overlay(alignment: .trailing) { Rectangle().fill(palette.line).frame(width: 1) }
            VStack(alignment: .leading, spacing: 0) {
                Text(look.label(.section, "Coder's rules")).ui(26, .bold, look)
                    .padding(.horizontal, 28).padding(.top, 48).padding(.bottom, 4)
                Text("Codex · 3 of these never come up: it doesn't ask before reads, searches or commands in its sandbox.")
                    .ui(13, .regular, look).foregroundStyle(palette.amber).padding(.horizontal, 28)
                Scroller {
                    RulesBody(name: "Coder", rules: Story.coder, agent: "Codex", hover: "5c")
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
