import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct LaneChatSheet: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<ChatFeature>
    internal let head: LaneHead
    internal let back: String
    internal let pop: () -> Void
    internal let done: () -> Void

    internal var body: some View {
        VStack(spacing: 0) {
            SheetBar(cancel: (back, pop), title: head.thread, done: "Done", run: done)
            LaneHeadView(head: head)
            ChatScreen(store: store, back: false, headed: false) {}
        }
        .background(look.palette.background).foregroundStyle(look.palette.foreground)
    }
}

internal struct LaneHeadView: View {
    @Environment(\.look) private var look
    internal let head: LaneHead

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 12) {
            if let lane = head.lane {
                Figure(figure: lane.source, title: lane.title).scaleEffect(look.figures == .sprite ? 1.6 : 1)
                    .frame(width: 64)
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 0) {
                        Text("\(lane.title) · ")
                        Text(lane.stage).foregroundStyle(palette.amber)
                    }
                    .ui(16, .semibold, look)
                    TimelineView(.everyMinute) { context in
                        Text(
                            [head.line, head.round.text(at: context.date) ?? ""].filter { !$0.isEmpty }.joined(
                                separator: " · "))
                    }
                    .ui(13, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
                }
            }
            Spacer(minLength: 4)
            if let pull = head.pull { LinkChip(link: pull) }
        }
        .padding(.horizontal, 16).padding(.vertical, 10)
        .overlay(alignment: .bottom) { Rectangle().fill(palette.line).frame(height: 0.5) }
    }
}

internal struct Figure: View {
    @Environment(\.look) private var look
    internal let figure: JSONValue
    internal var title = ""

    internal var body: some View {
        Acting(motion: Motion(tone: figure.string("state") ?? "")) { pose in drawn(pose.frame).acting(pose) }
    }

    @ViewBuilder private func drawn(_ frame: Int) -> some View {
        let palette = look.palette
        let lane = figure.string("state").flatMap(LaneState.init(rawValue:))
        if let lane, look.figures == .ascii {
            Text(lane.figure(frame: frame)).ui(13, .bold, look).foregroundStyle(palette.round(lane)).frame(width: 40)
        } else if look.figures == .sprite,
            let coat = figure.string("coat") ?? lane.map({ _ in SceneLane.coat(title) })
        {
            let pose = lane?.pose ?? "idle"
            let gait = lane == .working && look.moving ? "-gait\(frame)" : ""
            Sprite(name: "horse-\(coat)-\(pose)\(gait)", pack: "sprites.svg#s-\(pose)").frame(width: 40, height: 30)
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
