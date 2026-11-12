import SwiftUI

internal struct Metric: Identifiable {
    internal let id: String
    internal let value: String
    internal var bad = false
    internal let points: [Double]
    internal var range = ""
}

internal struct Machine: Identifiable {
    internal let id: String
    internal let seen: String
    internal var online = true
    internal var alerts: [String] = []
    internal var metrics: [Metric] = []
    internal var terminals = ""
    internal var open: String?
}

extension Machine {
    private static func wave(_ base: Double, _ swing: Double, _ phase: Double) -> [Double] {
        (0..<24).map {
            base + swing * sin(Double($0) / (2 + phase) + phase) + swing * 0.5 * cos(Double($0) * (0.9 + phase / 3))
        }
    }

    internal static let fixtures: [Machine] = [
        Machine(
            id: "mini", seen: "online",
            metrics: [
                Metric(id: "load/cpu", value: "0.42", points: wave(0.4, 0.15, 0)),
                Metric(id: "memory", value: "61%", points: wave(60, 8, 1)),
                Metric(id: "swap", value: "1.2 GB", points: wave(1.2, 0.2, 2)),
                Metric(id: "disk free", value: "48.2 GB", points: wave(48, 1, 3)),
            ],
            terminals: "3 working · 2 idle"),
        Machine(
            id: "studio", seen: "online", alerts: ["memory critical", "disk low"],
            metrics: [
                Metric(id: "load/cpu", value: "1.34", bad: true, points: wave(1.1, 0.3, 2)),
                Metric(
                    id: "memory", value: "94% critical", bad: true,
                    points: (0..<24).map { 38 + Double($0) * 2.2 + 4 * sin(Double($0)) },
                    range: "low 38% · high 94%"),
                Metric(id: "swap", value: "6.8 GB", points: wave(5, 1.5, 0)),
                Metric(id: "disk free", value: "9.1 GB low", bad: true, points: wave(10, 0.6, 1)),
            ],
            terminals: "5 working · 1 waiting", open: "memory"),
        Machine(id: "air", seen: "offline · last seen 3 h ago", online: false),
    ]
}

public struct MachinesView: View {
    @Environment(\.look) private var look
    private let machines = Machine.fixtures
    private let phone: Bool

    public var body: some View {
        SettingsPage(title: "Machines", phone: phone) {
            if phone {
                ForEach(machines) { MachineCard(machine: $0) }
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 320), alignment: .top)], spacing: 16) {
                    ForEach(machines) { MachineCard(machine: $0) }
                }
            }
        }
    }

    public init(phone: Bool = true) {
        self.phone = phone
    }
}

internal struct MachineCard: View {
    @Environment(\.look) private var look
    internal let machine: Machine

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 8) {
                Text(look.mono ? "*" : "●").font(.system(size: look.mono ? 14 : 11))
                    .foregroundStyle(machine.online ? palette.green : palette.mute)
                VStack(alignment: .leading, spacing: 1) {
                    Text(machine.id).ui(18, .bold, look)
                    Text(machine.seen).ui(12, .regular, look).foregroundStyle(palette.mute)
                }
                Spacer()
                if machine.online { ActionPill(title: look.mono ? "terminal" : "Terminal ›", primary: true) }
            }
            if !machine.alerts.isEmpty {
                Label(machine.alerts.joined(separator: " · "), systemImage: "exclamationmark.triangle.fill")
                    .ui(13, .semibold, look).foregroundStyle(palette.red)
            }
            ForEach(machine.metrics) { metric in
                MetricRow(metric: metric, open: metric.id == machine.open)
            }
            if !machine.terminals.isEmpty {
                Text("terminals: \(machine.terminals)").ui(13, .regular, look).foregroundStyle(palette.mute)
            }
        }
        .padding(16).surface(look, radius: 22).opacity(machine.online ? 1 : 0.7)
    }
}

internal struct MetricRow: View {
    @Environment(\.look) private var look
    internal let metric: Metric
    internal var open = false

    internal var body: some View {
        let palette = look.palette
        let tint = metric.bad ? palette.red : palette.cyan
        VStack(spacing: 6) {
            HStack(spacing: 10) {
                Text(metric.id).ui(13, .regular, look).foregroundStyle(palette.mute).frame(
                    width: 72, alignment: .leading)
                Text(metric.value).ui(15, .semibold, look).foregroundStyle(
                    metric.bad ? palette.red : palette.foreground)
                Spacer(minLength: 8)
                TrendLine(points: metric.points, tint: tint).frame(width: 90, height: 18)
            }
            if open {
                TrendLine(points: metric.points, tint: tint, fill: true).frame(height: 84).padding(.top, 4)
                HStack {
                    Text("24 h ago")
                    Spacer()
                    Text(metric.range)
                    Spacer()
                    Text("now")
                }
                .ui(11, .regular, look).foregroundStyle(palette.mute)
            }
        }
    }
}
