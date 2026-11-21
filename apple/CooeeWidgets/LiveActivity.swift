import ActivityKit
import CooeeCore
import CooeeUI
import SwiftUI
import WidgetKit

internal struct LiveActivityWidget: Widget {
    internal var body: some WidgetConfiguration {
        ActivityConfiguration(for: CooeeActivity.self) { context in
            LiveActivityCard(Self.activity(context.state)).surfaceLook(Entry.now.cache?.style)
                .activityBackgroundTint(.clear).widgetURL(SurfaceRow.url(context.state.thread))
        } dynamicIsland: { context in
            let activity = Self.activity(context.state)
            let style = Entry.now.cache?.style
            return DynamicIsland {
                DynamicIslandExpandedRegion(.bottom) { IslandExpanded(activity).surfaceLook(style) }
            } compactLeading: {
                IslandMark(activity.persona).surfaceLook(style)
            } compactTrailing: {
                IslandTrailing(activity).surfaceLook(style)
            } minimal: {
                IslandMark(activity.persona).surfaceLook(style)
            }
            .widgetURL(SurfaceRow.url(context.state.thread))
        }
    }

    private static func activity(_ shown: Shown) -> SurfaceActivity {
        SurfaceActivity(
            shown, personas: Entry.now.cache?.personas ?? [], now: Int(Date.now.timeIntervalSince1970 * 1000))
    }
}
