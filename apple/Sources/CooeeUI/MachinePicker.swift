import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct MachineDot: View {
    @Environment(\.look) private var look
    internal let dot: MachineRow.Dot

    internal var body: some View {
        let palette = look.palette
        Circle().fill(dot == .ready ? palette.green : dot == .warning ? palette.amber : palette.red)
            .frame(width: 7, height: 7)
    }
}

internal struct MachineMenu<Face: View>: View {
    internal let machines: [MachineRow]
    internal let pick: (String) -> Void
    @ViewBuilder internal let face: () -> Face

    internal var body: some View {
        Menu {
            Section("Computers") { ForEach(machines.computers) { item($0) } }
            if !machines.desks.isEmpty { Section("Desks") { ForEach(machines.desks) { item($0) } } }
        } label: {
            face()
        }
    }

    private func item(_ machine: MachineRow) -> some View {
        let symbol =
            switch machine.dot {
            case .ready: "circle.fill"
            case .warning: "exclamationmark.circle"
            case .offline: "circle.slash"
            }
        let badge = machine.desk ? " · \(machine.provider ?? "desk")" : ""
        return Button {
            pick(machine.name)
        } label: {
            Label(machine.title + badge, systemImage: symbol)
        }
    }
}

internal struct MachineChipLabel: View {
    @Environment(\.look) private var look
    internal let machine: MachineRow

    internal var body: some View {
        HStack(spacing: 4) {
            MachineDot(dot: machine.dot)
            Text(machine.title)
        }
        .ui(12, .regular, look).foregroundStyle(look.palette.mute).lineLimit(1)
        .padding(.horizontal, 9).padding(.vertical, 4)
        .background(look.palette.edge.opacity(0.35), in: .capsule)
    }
}

internal struct MachineSheet: View {
    internal let chip: MachineChip
    internal let started: Bool
    internal let pick: (String) -> Void
    internal let done: () -> Void

    internal var body: some View {
        NavigationStack {
            List {
                if let machine = chip.shown {
                    Section("Health") {
                        Label(machine.title, systemImage: "desktopcomputer")
                        Text(machine.health?.summary ?? "No sample yet").foregroundStyle(.secondary)
                    }
                    let problems = machine.problems + (chip.refusal.map { [$0] } ?? [])
                    if !problems.isEmpty {
                        Section("Not ready") { ForEach(problems, id: \.self) { Text($0).foregroundStyle(.red) } }
                    }
                }
                Section {
                    MachineMenu(machines: chip.machines.filter { $0.name != chip.current }, pick: pick) {
                        Text(started ? "Move…" : "Run on…")
                    }
                    if chip.moving { ProgressView("Moving…") }
                }
            }
            .navigationTitle("Machine")
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done", action: done) } }
        }
    }
}
