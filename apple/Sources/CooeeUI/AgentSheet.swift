import CooeeCore
import SwiftUI

internal struct GapLines: View {
    @Environment(\.look) private var look
    internal let gaps: [Gap]

    internal var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            ForEach(gaps, id: \.self) { gap in
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Glyph(symbol: "exclamationmark.triangle.fill", ascii: "!", size: 11)
                    Text(gap.words).fixedSize(horizontal: false, vertical: true)
                }
                .foregroundStyle(look.palette.amber)
            }
        }
        .ui(13, .regular, look)
    }
}

internal struct AgentLine: View {
    @Environment(\.look) private var look
    internal let agent: AgentOption
    internal let models: String
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
                    Text(models).ui(12, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
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
        .contentShape(Rectangle())
    }
}

internal struct AgentSheet: View {
    @Environment(\.look) private var look
    @Binding internal var draft: APIPersona
    internal let choices: PersonaChoices
    internal var pick: @MainActor @Sendable (String) -> Void = { _ in }
    internal var done: () -> Void = {}

    internal var body: some View {
        BottomSheet {
            RuleSheetBar(title: "\(draft.name)'s agent", done: "Done", cancel: done, run: done)
            Group2(header: "Agent") {
                if choices.agents.isEmpty { Text("No agents found: run cooee doctor on the Mac").padding(14) }
                ForEach(choices.agents) { agent in
                    Button {
                        pick(agent.id)
                    } label: {
                        AgentLine(
                            agent: agent, models: models(agent.id), picked: agent.id == draft.agent,
                            divider: agent.id != choices.agents.last?.id)
                    }
                    .buttonStyle(.plain).accessibilityLabel(agent.id)
                    .accessibilityValue(agent.gaps.map(\.words).joined(separator: " "))
                }
            }
            Group2 {
                let more = !choices.efforts.isEmpty
                row(("cpu", look.palette.purple), "Model", $draft.model, choices.models, divider: more)
                if !choices.efforts.isEmpty {
                    row(("gauge.medium", look.palette.cyan), "Effort", $draft.effort, choices.efforts, divider: false)
                }
            }
        }
    }

    private func models(_ agent: String) -> String {
        ModelList.picks(choices.lists, agent: agent).joined(separator: " · ")
    }

    @ViewBuilder private func row(
        _ mark: (symbol: String, tint: Color), _ title: String, _ value: Binding<String>, _ options: [String],
        divider: Bool
    ) -> some View {
        let (symbol, tint) = mark
        let shown = value.wrappedValue.isEmpty ? "Default" : value.wrappedValue
        let face = SettingRow(symbol: symbol, title: title, tint: tint, value: shown, divider: divider)
        if look.still {
            face
        } else {
            Menu {
                Picker(title, selection: value) {
                    Text("Default").tag("")
                    ForEach(options, id: \.self) { Text($0).tag($0) }
                }
                .pickerStyle(.inline)
            } label: {
                face.contentShape(Rectangle())
            }
            .buttonStyle(.plain).accessibilityLabel(title)
        }
    }
}

internal struct PersonaAgentRows: View {
    @Environment(\.look) private var look
    @Binding internal var draft: APIPersona
    internal let choices: PersonaChoices
    internal let pick: @MainActor @Sendable (String) -> Void
    internal let rules: () -> Void
    internal var rulesPicked = false
    @State private var choosing = false

    internal var body: some View {
        let palette = look.palette
        let gaps = choices.agents.first { $0.id == draft.agent }?.gaps.count ?? 0
        Button {
            choosing = true
        } label: {
            HStack {
                Text("Agent").ui(16, .regular, look)
                Spacer()
                if gaps > 0 { Text("\(gaps) gaps").ui(13, .regular, look).foregroundStyle(palette.amber) }
                AgentChip(persona: Persona(draft), effort: true)
            }
            .padding(.horizontal, 14).frame(height: 48).contentShape(Rectangle())
        }
        .buttonStyle(.plain).accessibilityLabel("Agent")
        .sheet(isPresented: $choosing) {
            AgentSheet(draft: $draft, choices: choices, pick: pick) { choosing = false }
                .presentationDetents([.medium, .large]).presentationBackground(.clear)
        }
        if let count = choices.rules {
            Button(action: rules) {
                SettingRow(
                    symbol: "checkmark.shield.fill", title: "Rules", tint: palette.green, value: "\(count)",
                    divider: !look.still
                )
                .contentShape(Rectangle()).background(rulesPicked ? palette.link.opacity(0.14) : .clear)
            }
            .buttonStyle(.plain).accessibilityLabel("Rules")
        }
    }
}
