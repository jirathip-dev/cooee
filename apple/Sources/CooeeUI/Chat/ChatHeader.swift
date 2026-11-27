import CooeeCore
import SwiftUI

internal struct ChatHeader: View {
    @Environment(\.look) private var look
    @Environment(\.asks) private var asks
    internal let persona: Persona
    internal let chips: [String]
    internal var back = true
    internal var plugged = true
    internal var actions = ChatActions()

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 10) {
                if let title = actions.backTo {
                    BackLink(title: title, run: actions.back).padding(.horizontal, -14)
                } else if back {
                    Button(action: actions.back) { RoundButton(symbol: "chevron.left", ascii: "<", size: 40) }
                        .buttonStyle(.plain).accessibilityLabel("Back")
                }
                sheet { FigureView(persona: persona, size: 38) }
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 6) {
                        sheet { Text(persona.name).ui(16, .semibold, look) }
                        if let sync = actions.sync { SyncBadge(sync: sync, asOf: actions.asOf) }
                    }
                    HStack(spacing: 6) {
                        chip
                        if let machine = actions.machine {
                            Button(action: actions.showMachine) { MachineChipLabel(machine: machine) }
                                .buttonStyle(.plain).accessibilityLabel("Machine \(machine.title)")
                        }
                    }
                }
                Spacer(minLength: 4)
                if let new = actions.new {
                    Button(action: new) { RoundButton(symbol: "plus", ascii: "+", size: 40) }
                        .buttonStyle(.plain).accessibilityLabel("New chat")
                }
                if let screen = actions.screen {
                    Button(action: screen) { RoundButton(symbol: "display", ascii: "scr", size: 40) }
                        .buttonStyle(.plain).accessibilityLabel("Screen")
                }
                if let call = actions.voice.call {
                    Button(action: call) { RoundButton(symbol: "phone.fill", ascii: "c", size: 40) }
                        .buttonStyle(.plain).accessibilityLabel("Call")
                }
                more
            }
            if let status = actions.status { LaneStatusRow(status: status, close: actions.close) }
            if plugged && (!chips.isEmpty || actions.workflow != nil || asks.count > 0) { plugins }
        }
        .padding(.horizontal, 14).padding(.vertical, 8)
        .overlay(alignment: .bottom) { Rectangle().fill(palette.line).frame(height: 0.5) }
    }

    private func sheet(_ label: () -> some View) -> some View {
        Button {
            actions.persona?()
        } label: {
            label()
        }
        .buttonStyle(.plain).accessibilityLabel(persona.name)
    }

    @ViewBuilder private var chip: some View {
        if let choices = actions.choices {
            AgentMenu(persona: persona, choices: choices, effort: false, pick: actions.agent, set: actions.model)
        } else {
            AgentChip(persona: persona)
        }
    }

    private var more: some View {
        Button(action: actions.more) { RoundButton(symbol: "ellipsis", ascii: "…", size: 40) }
            .buttonStyle(.plain).accessibilityLabel("More")
    }

    private var plugins: some View {
        PluginChips(chips: chips, actions: actions).padding(.leading, back ? 50 : 0)
    }
}
