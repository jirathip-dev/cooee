import CooeeCore
import SwiftUI

internal struct RowView: View {
    @Environment(\.look) private var look
    @Environment(\.act) private var act
    @State private var pending: JSONValue?
    @State private var open: Bool?
    internal let row: JSONValue

    private var rounds: [JSONValue] { row.list("rounds") }
    private var unfolded: Bool { !rounds.isEmpty && (open ?? (rounds.last?.string("state") == "needs")) }

    internal var body: some View {
        Group {
            if !rounds.isEmpty {
                VStack(alignment: .leading, spacing: 0) {
                    Button {
                        open = !unfolded
                    } label: {
                        line
                    }
                    .buttonStyle(.plain)
                    if unfolded { RoundsView(rounds: rounds) }
                }
            } else if row["tap"] == nil {
                line
            } else {
                ActButton(source: row) { line }
            }
        }
        .contextMenu {
            ForEach(Array(row.list("swipe").enumerated()), id: \.offset) { _, swipe in
                Button(swipe.string("label") ?? "", role: swipe.string("tone") == "bad" ? .destructive : nil) {
                    if Confirm(for: swipe) == nil { run(swipe) } else { pending = swipe }
                }
            }
        }
        .modifier(Confirming(pending: $pending, run: run))
    }

    private var line: some View {
        let palette = look.palette
        let subtitle = unfolded ? nil : row["subtitle"]
        return HStack(spacing: 10) {
            if let figure = row["figure"] { Figure(figure: figure, title: row.string("title") ?? "") }
            VStack(alignment: .leading, spacing: 2) {
                Text(row.string("title") ?? "").ui(15, .medium, look).lineLimit(1)
                    .foregroundStyle(
                        row.string("style") == "destructive"
                            ? palette.red : row.string("style") == "link" ? palette.link : palette.foreground)
                if let text = subtitle?.string ?? subtitle?.string("text") {
                    Text(text).ui(12, .regular, look).foregroundStyle(
                        palette.tone(subtitle?.string("tone")) ?? palette.mute
                    ).lineLimit(1)
                }
            }
            Spacer(minLength: 4)
            if let badge = row["badge"] { Pill(badge: badge) }
            if let value = row["value"] {
                RelativeValue(value: value).ui(13, .regular, look).foregroundStyle(palette.mute)
            }
            if let button = row["button"] { RowButton(button: button) }
            trailing
        }
        .padding(.leading, CGFloat(row["indent"]?.number ?? 0) * 18).padding(.horizontal, 12).frame(minHeight: 44)
        .contentShape(.rect)
        .overlay(alignment: .leading) {
            if row["changed"]?.bool == true {
                RoundedRectangle(cornerRadius: 2).fill(palette.lead).frame(width: 3).padding(.vertical, 10)
            }
        }
        .modifier(Lifted(lifted: row["lifted"]?.bool == true))
    }

    @ViewBuilder private var trailing: some View {
        if row["handle"]?.bool == true {
            Image(systemName: "line.3.horizontal").font(.system(size: 15, weight: .semibold))
                .foregroundStyle(look.palette.mute).accessibilityLabel("Reorder")
        } else if !rounds.isEmpty || (row["tap"] != nil && row.string("accessory") != "none") {
            Image(systemName: unfolded ? "chevron.down" : "chevron.right").font(.system(size: 11, weight: .bold))
                .foregroundStyle(look.palette.mute)
        }
    }

    private func run(_ swipe: JSONValue) {
        if let action = BlockAction(action: nil, source: swipe, values: [:]) { act(action) }
    }
}

internal struct RelativeValue: View {
    internal let value: JSONValue

    internal var body: some View {
        TimelineView(.everyMinute) { context in Text(value.text(at: context.date) ?? "") }
    }
}

internal struct Figure: View {
    @Environment(\.look) private var look
    internal let figure: JSONValue
    internal var title = ""

    internal var body: some View {
        let palette = look.palette
        let lane = figure.string("state").flatMap(LaneState.init(rawValue:))
        if let lane, look.style == .terminal {
            Text(lane.figure).ui(13, .bold, look).foregroundStyle(palette.round(lane)).frame(width: 40)
        } else if look.style == .scene, let coat = figure.string("coat") ?? lane.map({ _ in SceneLane.coat(title) }) {
            Sprite(name: "horse-\(coat)-\(lane?.pose ?? "idle")").frame(width: 40, height: 30)
        } else if let lane {
            Sprite(name: "row-\(sprite(lane))").frame(width: 22, height: 22).frame(width: 28)
        } else if let icon = figure.string("icon") {
            Image(systemName: icon).font(.system(size: 14, weight: .semibold)).foregroundStyle(palette.link).frame(
                width: 28)
        } else if let avatar = figure.string("avatar") {
            Text(avatar.prefix(2).uppercased()).ui(12, .bold, look).frame(width: 28, height: 28)
                .background(palette.edge.opacity(0.5), in: .circle)
        } else {
            Circle().fill(palette.tone(figure.string("state")) ?? palette.mute).frame(width: 10, height: 10).frame(
                width: 28)
        }
    }

    private func sprite(_ lane: LaneState) -> String {
        guard lane == .working else { return lane.rawValue }
        return ["read", "run"].first { $0 == figure.string("tool") } ?? "edit"
    }
}

internal struct RowButton: View {
    @Environment(\.look) private var look
    internal let button: JSONValue

    internal var body: some View {
        let tint = look.palette.tone(button.string("tone")) ?? look.palette.link
        ActButton(source: button) {
            Text(button.string("label") ?? "").ui(13, .semibold, look).foregroundStyle(tint)
                .padding(.horizontal, 12).frame(minHeight: 30)
                .background(look.mono ? Color.clear : tint.opacity(0.16), in: .capsule)
        }
    }
}

internal struct Lifted: ViewModifier {
    @Environment(\.look) private var look
    internal let lifted: Bool

    @ViewBuilder
    internal func body(content: Content) -> some View {
        if lifted {
            content.background(look.palette.background, in: .rect(cornerRadius: 12))
                .shadow(color: .black.opacity(0.4), radius: 12, y: 8).scaleEffect(1.02).offset(y: -6).zIndex(1)
        } else {
            content
        }
    }
}
