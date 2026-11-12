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
                Figure(figure: lane.source, title: lane.title).scaleEffect(look.style == .scene ? 1.6 : 1)
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
            if let pull = head.pull { RowButton(button: pull) }
        }
        .padding(.horizontal, 16).padding(.vertical, 10)
        .overlay(alignment: .bottom) { Rectangle().fill(palette.line).frame(height: 0.5) }
    }
}
