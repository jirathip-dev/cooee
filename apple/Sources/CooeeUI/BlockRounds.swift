import CooeeCore
import SwiftUI

extension Palette {
    internal func round(_ state: LaneState) -> Color {
        switch state {
        case .done, .merged: green
        case .fail, .needs: red
        case .working: amber
        case .idle: mute
        }
    }
}

private struct Rail: Shape {
    fileprivate func path(in rect: CGRect) -> Path {
        Path { path in
            path.move(to: CGPoint(x: rect.midX, y: rect.minY))
            path.addLine(to: CGPoint(x: rect.midX, y: rect.maxY))
        }
    }
}

internal struct RoundsView: View {
    @Environment(\.look) private var look
    internal let rounds: [JSONValue]

    internal var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            ForEach(Array(rounds.enumerated()), id: \.offset) { index, round in
                RoundView(round: round, current: index == rounds.count - 1)
            }
        }
        .padding(.leading, 22).padding(.trailing, 12).padding(.bottom, 12)
        .background(alignment: .leading) {
            Rail().stroke(look.palette.edge, style: StrokeStyle(lineWidth: 1.5, dash: look.mono ? [4, 3] : []))
                .frame(width: 20).padding(.leading, 22).padding(.vertical, 12)
        }
    }
}

internal struct RoundView: View {
    @Environment(\.look) private var look
    internal let round: JSONValue
    internal let current: Bool

    private var state: LaneState { LaneState(rawValue: round.string("state") ?? "") ?? .idle }

    internal var body: some View {
        HStack(alignment: .top, spacing: 10) {
            marker.frame(width: 20, height: 34)
            VStack(alignment: .leading, spacing: 6) {
                if current || round["tap"] == nil { line } else { ActButton(source: round) { line } }
                if current { RoundBody(round: round, state: state) }
            }
        }
    }

    @ViewBuilder private var marker: some View {
        let tint = look.palette.round(state)
        if look.mono {
            let glyphs: [LaneState: String] = [.done: "+", .fail: "x", .working: ">", .needs: "!"]
            Text(glyphs[state] ?? "-").ui(15, .bold, look).foregroundStyle(tint)
        } else {
            Circle().fill(tint).frame(width: current ? 14 : 10, height: current ? 14 : 10)
        }
    }

    private var line: some View {
        let palette = look.palette
        return HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text(round.string("name") ?? "").ui(15, .semibold, look).lineLimit(1).layoutPriority(1)
            Text(round.string("summary") ?? "").ui(13, .regular, look).foregroundStyle(palette.round(state))
                .lineLimit(1)
            Spacer(minLength: 4)
            if let since = round["since"] {
                RelativeValue(value: .object(["since": since])).ui(13, .regular, look).foregroundStyle(palette.mute)
            }
            if !current && round["tap"] != nil {
                Image(systemName: "chevron.right").font(.system(size: 11, weight: .bold)).foregroundStyle(palette.mute)
            }
        }
        .frame(minHeight: 34).contentShape(.rect)
    }
}

internal struct RoundBody: View {
    @Environment(\.look) private var look
    internal let round: JSONValue
    internal let state: LaneState

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 10) {
            if let lane = round["lane"] { self.lane(lane) }
            if !round.list("blocks").isEmpty { BlockStack(blocks: round.list("blocks")) }
            if !round.list("links").isEmpty { links }
        }
        .padding(12).frame(maxWidth: .infinity, alignment: .leading)
        .overlay(
            RoundedRectangle(cornerRadius: look.mono ? 4 : 14)
                .stroke(state == .needs ? palette.red.opacity(0.7) : palette.edge, lineWidth: 1)
        )
    }

    private func lane(_ lane: JSONValue) -> some View {
        let palette = look.palette
        let head = ["name", "harness", "model", "host"].compactMap { lane.string($0) }.joined(separator: " · ")
        let lines = lane.list("lines").compactMap(\.string)
        return VStack(alignment: .leading, spacing: 4) {
            Text(head).ui(13, .regular, look).foregroundStyle(palette.mute)
            ForEach(Array(lines.enumerated()), id: \.offset) { index, text in
                let live = state == .working && index == lines.count - 1
                Text(live ? "▍ \(text)" : text).font(.system(size: 13, design: .monospaced)).lineLimit(1)
                    .foregroundStyle(live ? palette.amber : palette.mute)
            }
        }
    }

    private var links: some View {
        let palette = look.palette
        return HStack(spacing: 8) {
            ForEach(Array(round.list("links").enumerated()), id: \.offset) { _, link in
                ActButton(source: link) {
                    Text("\(link.string("label") ?? "") ›").ui(14, .medium, look).foregroundStyle(palette.link)
                        .padding(.horizontal, 12).frame(height: 32)
                        .overlay(RoundedRectangle(cornerRadius: look.mono ? 4 : 16).stroke(palette.edge, lineWidth: 1))
                }
            }
        }
    }
}
