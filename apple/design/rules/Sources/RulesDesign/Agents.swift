import CooeeCore
@testable import CooeeUI
import SwiftUI

internal struct GapLines: View {
    @Environment(\.look) private var look
    internal let gaps: [String]

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 4) {
            if gaps.isEmpty {
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Glyph(symbol: "checkmark.circle.fill", ascii: "ok", size: 11)
                    Text("Asks before everything; your rules answer it")
                }
                .foregroundStyle(palette.green)
            }
            ForEach(gaps, id: \.self) { gap in
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Glyph(symbol: "exclamationmark.triangle.fill", ascii: "!", size: 11)
                    Text(gap).fixedSize(horizontal: false, vertical: true)
                }
                .foregroundStyle(palette.amber)
            }
        }
        .ui(13, .regular, look)
    }
}

internal struct AgentLine: View {
    @Environment(\.look) private var look
    internal let agent: AgentFixture
    internal let picked: Bool
    internal var divider = true

    internal var body: some View {
        let palette = look.palette
        HStack(alignment: .top, spacing: 12) {
            Mark(text: picked ? "(x)" : "( )", size: 14) {
                Image(systemName: picked ? "checkmark.circle.fill" : "circle").font(.system(size: 20))
                    .foregroundStyle(picked ? palette.link : palette.mute)
            }
            .frame(width: 26)
            VStack(alignment: .leading, spacing: 6) {
                HStack(alignment: .firstTextBaseline) {
                    Text(agent.id).ui(16, .semibold, look)
                    Spacer()
                    Text(agent.models).ui(12, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
                }
                GapLines(gaps: agent.gaps)
            }
        }
        .padding(.horizontal, 14).padding(.vertical, 12)
        .background(picked ? palette.link.opacity(0.08) : .clear)
        .overlay(alignment: .bottom) {
            if divider {
                Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, look.textMarks ? 0 : 52)
            }
        }
    }
}

internal struct AgentSheet: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        BottomSheet {
            SheetBar(title: "Coder's agent", done: "Done")
            VStack(alignment: .leading, spacing: 8) {
                Group2(header: "Agent") {
                    ForEach(Story.agents) { agent in
                        AgentLine(agent: agent, picked: agent.id == "Codex", divider: agent.id != "Gemini")
                    }
                }
                Text("A gap is something cooee can't tell that agent. Your rules still answer whatever it asks.")
                    .ui(12.5, .regular, look).foregroundStyle(palette.mute).padding(.horizontal, 16)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Group2 {
                SettingRow(symbol: "cpu", title: "Model", tint: palette.purple, value: "gpt-5.5")
                SettingRow(symbol: "gauge.medium", title: "Effort", tint: palette.cyan, value: "medium", divider: false)
            }
        }
    }
}

internal struct PersonaSheet: View {
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
                    Line(divider: false) {
                        Text("You work on cooee. Run the tests before you say it's done.").ui(16, .regular, look)
                    }
                }
                Group2(header: "Agent") {
                    SettingRow(
                        symbol: "sparkles", title: "Codex · gpt-5.5", tint: palette.lead, value: "4 gaps", warn: true)
                    SettingRow(symbol: "checkmark.shield.fill", title: "Rules", tint: palette.green, value: "5", divider: false)
                        .background(rulesPicked ? palette.link.opacity(0.14) : .clear)
                }
                Group2 {
                    SettingRow(symbol: "folder.fill", title: "Folder", tint: palette.amber, value: "~/src/cooee")
                    SettingRow(
                        symbol: "desktopcomputer", title: "Machine", tint: palette.cyan, value: "studio", divider: false)
                }
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 16)
        }
        .background(look.outline ? palette.background : palette.card.opacity(0.35))
        .foregroundStyle(palette.foreground)
    }
}
