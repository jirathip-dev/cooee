import CooeeCore
import SwiftUI

internal struct PersonaBlocks: View {
    @Environment(\.look) internal var look
    internal let screen: JSONValue
    internal var open: (Int) -> Void = { _ in }

    internal var body: some View {
        VStack(spacing: 18) {
            ForEach(Array(screen.list("blocks").enumerated()), id: \.offset) { _, block in
                switch block.type {
                case "screen" where !(block.string("url") ?? "").isEmpty: live(block)
                case "terminal" where !block.list("lines").isEmpty: terminal(block)
                case "workflow" where !block.list("issues").isEmpty: workflow(block)
                case "routines": routines(block.list("rows"))
                case "places" where !block.list("rows").isEmpty: places(block.list("rows"))
                default: EmptyView()
                }
            }
        }
    }

    private func workflow(_ block: JSONValue) -> some View {
        let issues = block.list("issues")
        return VStack(alignment: .leading, spacing: 6) {
            heading("Workflow") { mapToggle }
            Group2 {
                ForEach(Array(issues.enumerated()), id: \.offset) { _, issue in issueRow(issue) }
                more("All \(issues.count + Int(block["more"]?.number ?? 0)) issues")
            }
        }
    }

    @ViewBuilder private var mapToggle: some View {
        if look.mono {
            Text("[list] map").ui(11, .semibold, look).foregroundStyle(look.palette.mute)
        } else {
            HStack(spacing: 0) {
                ForEach(["list.bullet", "point.3.connected.trianglepath.dotted"], id: \.self) { symbol in
                    Image(systemName: symbol).frame(width: 30, height: 22)
                        .background(symbol == "list.bullet" ? look.palette.card : .clear, in: .capsule)
                }
            }
            .font(.system(size: 11, weight: .semibold)).padding(2).background(
                look.palette.edge.opacity(0.35), in: .capsule)
        }
    }

    private func issueRow(_ issue: JSONValue) -> some View {
        let palette = look.palette
        let tone = palette.tone(issue.string("tone")) ?? palette.mute
        return HStack(spacing: 10) {
            Circle().fill(tone).frame(width: 8, height: 8)
            Text("#\(Int(issue["number"]?.number ?? 0))").ui(14, .semibold, look).monospacedDigit()
            Text(issue.string("title") ?? "").ui(14, .regular, look).lineLimit(1)
            Spacer(minLength: 4)
            Text(issue.string("stage") ?? "").ui(12, .medium, look).foregroundStyle(tone).lineLimit(1)
        }
        .padding(.horizontal, 14).frame(height: 44)
        .overlay(alignment: .bottom) { rule(true) }
    }
}

extension PersonaBlocks {
    private func routines(_ rows: [JSONValue]) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            heading("Routines") {
                Text(look.mono ? "[+ add]" : "+ Add").ui(14, .semibold, look).foregroundStyle(look.palette.link)
            }
            if !rows.isEmpty {
                Group2 {
                    ForEach(Array(rows.enumerated()), id: \.offset) { index, routine in
                        routineRow(routine, divider: index < rows.count - 1)
                    }
                }
            }
        }
    }

    private func routineRow(_ routine: JSONValue, divider: Bool) -> some View {
        let palette = look.palette
        let isOn = routine["on"]?.bool ?? false
        let when = [routine.string("when") ?? "", isOn ? routine.string("next") ?? "" : "paused"]
        return HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(routine.string("name") ?? "").ui(15, .medium, look)
                Text(when.filter { !$0.isEmpty }.joined(separator: " · "))
                    .ui(12, .regular, look).foregroundStyle(isOn ? palette.mute : palette.amber).lineLimit(1)
            }
            Spacer(minLength: 4)
            if look.mono {
                Text(isOn ? "[on ]" : "[off]").ui(13, .bold, look).foregroundStyle(palette.mute)
            } else {
                Capsule().fill(isOn ? palette.green : palette.edge.opacity(0.6)).frame(width: 46, height: 28)
                    .overlay(alignment: isOn ? .trailing : .leading) { Circle().fill(.white).padding(2) }
            }
        }
        .padding(.horizontal, 14).padding(.vertical, 9)
        .overlay(alignment: .bottom) { rule(divider) }
        .opacity(isOn ? 1 : 0.75)
    }

    private func places(_ rows: [JSONValue]) -> some View {
        Group2(header: "Places") {
            ForEach(Array(rows.enumerated()), id: \.offset) { index, place in
                Button {
                    open(index)
                } label: {
                    SettingRow(
                        symbol: place.string("symbol") ?? "puzzlepiece.extension.fill",
                        title: place.string("title") ?? "", tint: look.palette.green,
                        value: place.string("value") ?? "", divider: index < rows.count - 1
                    )
                }
                .buttonStyle(.plain)
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
