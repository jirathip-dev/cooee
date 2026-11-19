import CooeeCore
import CooeeUI
import SwiftUI
import WidgetKit

@main
internal struct CooeeWidgets: WidgetBundle {
    internal var body: some Widget {
        NeedsWidget()
        RunningWidget()
        GlanceWidget()
        LiveActivityWidget()
    }
}

internal struct Entry: TimelineEntry {
    internal let date: Date
    internal let cache: GlanceCache?

    internal var glance: SurfaceGlance {
        SurfaceGlance(
            cache ?? GlanceCache(glance: Glance(), personas: [], host: ""), now: Int(date.timeIntervalSince1970 * 1000))
    }

    internal static var now: Entry {
        let folder = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: GlanceCache.group)
        return Entry(date: .now, cache: GlanceCache.read(GlanceCache.file(in: folder)))
    }
}

internal struct Provider: TimelineProvider {
    internal func placeholder(in context: Context) -> Entry { Entry(date: .now, cache: nil) }

    internal func getSnapshot(in context: Context, completion: @escaping (Entry) -> Void) { completion(.now) }

    internal func getTimeline(in context: Context, completion: @escaping (Timeline<Entry>) -> Void) {
        completion(Timeline(entries: [.now], policy: .after(.now.addingTimeInterval(15 * 60))))
    }
}

internal struct Card<Content: View>: View {
    internal let entry: Entry
    @ViewBuilder internal let content: Content

    internal var body: some View {
        content.surfaceLook(entry.cache?.style).containerBackground(.background, for: .widget)
    }
}

internal struct NeedsWidget: Widget {
    internal var body: some WidgetConfiguration {
        StaticConfiguration(kind: "needs", provider: Provider()) { entry in
            Card(entry: entry) { Needs(glance: entry.glance) }
        }
        .configurationDisplayName("Needs you")
        .description("The asks and blocked lanes waiting on you. A tap opens the top one.")
        .supportedFamilies([.systemSmall, .accessoryInline, .accessoryCircular, .accessoryRectangular])
    }
}

internal struct Needs: View {
    @Environment(\.widgetFamily) private var family
    internal let glance: SurfaceGlance

    internal var body: some View {
        Group {
            switch family {
            case .accessoryInline: Text("\(glance.needs.count) need you")
            case .accessoryCircular: LockCircle(glance.needs.count)
            case .accessoryRectangular: LockRect(glance.top)
            default: NeedsSmall(glance)
            }
        }
        .widgetURL(glance.top?.url)
    }
}

internal struct RunningWidget: Widget {
    internal var body: some WidgetConfiguration {
        StaticConfiguration(kind: "running", provider: Provider()) { entry in
            Card(entry: entry) { RunningSmall(entry.glance).widgetURL(entry.glance.running.first?.url) }
        }
        .configurationDisplayName("Running")
        .description("The turns and lanes running, with their clocks.")
        .supportedFamilies([.systemSmall])
    }
}

internal struct GlanceWidget: Widget {
    internal var body: some WidgetConfiguration {
        StaticConfiguration(kind: "glance", provider: Provider()) { entry in
            Card(entry: entry) { GlanceLarge(entry.glance) }
        }
        .configurationDisplayName("cooee")
        .description("What needs you over what runs. Each row opens its thread or ask.")
        .supportedFamilies([.systemLarge])
    }
}
