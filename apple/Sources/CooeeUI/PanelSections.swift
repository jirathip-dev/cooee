import SwiftUI

internal struct PanelSections: View {
    @Environment(\.look) internal var look
    internal let content: PanelContent

    internal var body: some View {
        VStack(spacing: 18) {
            if !content.screen.isEmpty { screen }
            if !content.terminal.isEmpty { terminal }
            if !content.issues.isEmpty { workflow }
            routines
            if !content.places.isEmpty { places }
        }
    }

    private var workflow: some View {
        VStack(alignment: .leading, spacing: 6) {
            heading("Workflow") { mapToggle }
            Group2 {
                ForEach(content.issues, id: \.number) { issue in
                    issueRow(issue, divider: true)
                }
                more("All \(content.issues.count + content.moreIssues) issues")
            }
        }
    }

    private var mapToggle: some View {
        let palette = look.palette
        return HStack(spacing: 0) {
            toggle("list.bullet", "list", isOn: true)
            toggle("point.3.connected.trianglepath.dotted", "map", isOn: false)
        }
        .padding(2).background(palette.edge.opacity(0.35), in: .capsule)
    }

    private func toggle(_ symbol: String, _ ascii: String, isOn: Bool) -> some View {
        let palette = look.palette
        return Group {
            if look.mono { Text(isOn ? "[\(ascii)]" : ascii) } else { Image(systemName: symbol) }
        }
        .ui(11, .semibold, look).foregroundStyle(isOn ? palette.foreground : palette.mute)
        .frame(width: look.mono ? 44 : 30, height: 22)
        .background(isOn && !look.mono ? palette.card : .clear, in: .capsule)
    }

    private func issueRow(_ issue: PanelIssue, divider: Bool) -> some View {
        let palette = look.palette
        return HStack(spacing: 10) {
            Circle().fill(issue.tone.color(palette)).frame(width: 8, height: 8)
            Text("#\(issue.number)").ui(14, .semibold, look).monospacedDigit()
            Text(issue.title).ui(14, .regular, look).lineLimit(1)
            Spacer(minLength: 4)
            Text(issue.stage).ui(12, .medium, look).foregroundStyle(issue.tone.color(palette)).lineLimit(1)
        }
        .padding(.horizontal, 14).frame(height: 44)
        .overlay(alignment: .bottom) { rule(divider) }
    }
}

extension PanelSections {
    private var routines: some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 6) {
            heading("Routines") {
                Text(look.mono ? "[+ add]" : "+ Add").ui(14, .semibold, look).foregroundStyle(palette.link)
            }
            if !content.routines.isEmpty {
                Group2 {
                    ForEach(Array(content.routines.enumerated()), id: \.offset) { index, routine in
                        routineRow(routine, divider: index < content.routines.count - 1)
                    }
                }
            }
        }
    }

    private func routineRow(_ routine: PanelRoutine, divider: Bool) -> some View {
        let palette = look.palette
        return HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(routine.name).ui(15, .medium, look)
                Text("\(routine.when) · \(routine.isOn ? routine.next : "paused")")
                    .ui(12, .regular, look).foregroundStyle(routine.isOn ? palette.mute : palette.amber).lineLimit(1)
            }
            Spacer(minLength: 4)
            Switch(isOn: routine.isOn)
        }
        .padding(.horizontal, 14).padding(.vertical, 9)
        .overlay(alignment: .bottom) { rule(divider) }
        .opacity(routine.isOn ? 1 : 0.75)
    }

    private var places: some View {
        let palette = look.palette
        return Group2(header: "Places") {
            ForEach(Array(content.places.enumerated()), id: \.offset) { index, place in
                HStack(spacing: 12) {
                    if !look.mono {
                        Image(systemName: place.symbol).font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(.white).frame(width: 30, height: 30)
                            .background(palette.green, in: .rect(cornerRadius: 8))
                    }
                    Text(place.name).ui(16, .regular, look)
                    Spacer()
                    Badge(count: place.count)
                    Image(systemName: "chevron.right").font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(palette.mute)
                }
                .padding(.horizontal, 14).frame(height: 48)
                .overlay(alignment: .bottom) { rule(index < content.places.count - 1) }
            }
        }
    }

    private func heading(_ title: String, @ViewBuilder trailing: () -> some View) -> some View {
        HStack {
            Text(look.mono ? "# \(title.lowercased())" : title.uppercased()).ui(12, .medium, look)
                .foregroundStyle(look.palette.mute)
            Spacer()
            trailing()
        }
        .padding(.leading, 16).padding(.trailing, 4)
    }

    internal func more(_ title: String) -> some View {
        HStack {
            Text(title).ui(15, .regular, look).foregroundStyle(look.palette.link)
            Spacer()
            Image(systemName: "chevron.right").font(.system(size: 12, weight: .semibold))
                .foregroundStyle(look.palette.mute)
        }
        .padding(.horizontal, 14).frame(height: 44)
    }

    @ViewBuilder internal func rule(_ shown: Bool) -> some View {
        if shown { Rectangle().fill(look.palette.line).frame(height: 0.5).padding(.leading, 14) }
    }
}

internal struct Switch: View {
    @Environment(\.look) private var look
    internal let isOn: Bool

    internal var body: some View {
        let palette = look.palette
        if look.mono {
            Text(isOn ? "[on ]" : "[off]").ui(13, .bold, look).foregroundStyle(isOn ? palette.green : palette.mute)
        } else {
            Capsule().fill(isOn ? palette.green : palette.edge.opacity(0.6)).frame(width: 46, height: 28)
                .overlay(alignment: isOn ? .trailing : .leading) {
                    Circle().fill(.white).frame(width: 24, height: 24).padding(2)
                        .shadow(color: .black.opacity(0.15), radius: 2, y: 1)
                }
        }
    }
}
