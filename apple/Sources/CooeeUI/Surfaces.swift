import AppIntents
import CooeeCore
import CooeeFeatures
import SwiftUI

/// One line of a widget: a thread or a lane that needs you or runs; a tap opens `thread`.
public struct SurfaceRow: Identifiable, Sendable {
    public let thread: String
    public let persona: Persona
    public let title: String
    public let detail: String
    public let trailing: String

    public var id: String { thread }
    public var url: URL { SurfaceRow.url(thread) }

    public init(thread: String, persona: Persona, title: String, detail: String, trailing: String) {
        self.thread = thread
        self.persona = persona
        self.title = title
        self.detail = detail
        self.trailing = trailing
    }

    /// The link that opens `thread` in the app.
    public static func url(_ thread: String) -> URL {
        URL(string: "cooee://thread/")?.appending(path: thread) ?? URL(filePath: "/")
    }
}

/// The one Live Activity as drawn: the persona, the headline, the thread, the clock, the line and the rest.
public struct SurfaceActivity: Sendable {
    public let thread: String
    public let persona: Persona
    public let headline: String
    public let place: String
    public let clock: String
    public let line: String
    public let foot: String
    public let more: String
    public let tools: Int
    public var start: Date?

    public var asks: Bool { persona.state == .needs }
    public var running: Bool { persona.state == .working }

    public init(
        thread: String, persona: Persona, headline: String, place: String, clock: String, line: String, foot: String,
        more: String, tools: Int = 0
    ) {
        self.thread = thread
        self.persona = persona
        self.headline = headline
        self.place = place
        self.clock = clock
        self.line = line
        self.foot = foot
        self.more = more
        self.tools = tools
    }
}

extension Persona {
    /// `id`'s persona among `personas` in `state`, else one named `name`.
    public static func of(_ id: String, _ name: String, _ state: LaneState, in personas: [APIPersona]) -> Persona {
        let api = personas.first { $0.id == id } ?? APIPersona(id: id, name: name, agent: "")
        return Persona(api, state: state)
    }
}

internal struct IslandLook: ViewModifier {
    @Environment(\.look) private var look

    internal func body(content: Content) -> some View {
        var dark = look
        dark.palette = .farmDark
        return content.environment(\.look, dark).environment(\.colorScheme, .dark)
            .foregroundStyle(Palette.farmDark.foreground)
    }
}

internal struct Kicker: View {
    @Environment(\.look) private var look
    internal let text: String
    internal var tint: Color?

    internal var body: some View {
        let mono = look.type == .mono
        Text(mono ? "# \(text.lowercased())" : text.uppercased()).ui(11, .bold, look)
            .foregroundStyle(tint ?? look.palette.mute).tracking(mono ? 0 : 0.6)
    }
}

internal struct StopButton: View {
    @Environment(\.look) private var look
    internal let thread: String

    internal var body: some View {
        let (red, mono) = (look.palette.red, look.type == .mono)
        Button(intent: StopTurnIntent(thread: thread)) {
            HStack(spacing: 5) {
                if !mono { Image(systemName: "stop.fill").font(.system(size: 10)) }
                Text(mono ? "[■ Stop]" : "Stop")
            }
            .fixedSize().ui(13, .semibold, look).foregroundStyle(red).padding(.horizontal, 10).frame(height: 28)
            .background(red.opacity(mono ? 0 : 0.16), in: mono ? AnyShape(Rectangle()) : AnyShape(Capsule()))
        }
        .buttonStyle(.plain)
    }
}

internal struct Count: View {
    @Environment(\.look) private var look
    internal let text: String

    internal var body: some View {
        let mono = look.type == .mono
        Text(mono ? "[\(text)]" : text).ui(12, .semibold, look).foregroundStyle(look.palette.mute)
            .padding(.horizontal, 8).frame(height: 22)
            .background(mono ? Color.clear : look.palette.edge.opacity(0.35), in: .capsule)
    }
}

internal struct StateMark: View {
    @Environment(\.look) private var look
    internal let state: LaneState

    internal var body: some View {
        let palette = look.palette
        let color = state == .needs || state == .fail ? palette.red : state == .working ? palette.cyan : palette.green
        if look.type == .mono {
            Text(state.figure).ui(12, .bold, look).foregroundStyle(color)
        } else {
            Circle().fill(color).frame(width: 8, height: 8)
        }
    }
}

internal struct ClockText: View {
    internal let activity: SurfaceActivity

    internal var body: some View {
        if activity.running, let start = activity.start {
            Text(timerInterval: start...Date.distantFuture, countsDown: false)
        } else {
            Text(activity.clock)
        }
    }
}

/// The Live Activity on the lock screen: one tap target that opens the thread or the ask; Stop on a running turn.
public struct LiveActivityCard: View {
    @Environment(\.look) private var look
    private let activity: SurfaceActivity

    public var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .top, spacing: 10) {
                FigureView(persona: activity.persona, size: 40)
                VStack(alignment: .leading, spacing: 2) {
                    Text(activity.headline).ui(16, .semibold, look)
                    Text(activity.place).ui(13, .regular, look).foregroundStyle(palette.mute)
                }
                Spacer(minLength: 4)
                ClockText(activity: activity).ui(15, .semibold, look).monospacedDigit()
                    .foregroundStyle(activity.asks ? palette.red : palette.cyan)
            }
            Text(activity.line).font(
                .system(size: 13, design: activity.asks && look.type != .mono ? .default : .monospaced)
            ).lineLimit(2)
                .foregroundStyle(activity.asks ? palette.foreground : palette.mute)
            HStack(spacing: 8) {
                StateMark(state: activity.persona.state)
                Text(activity.foot).ui(13, .medium, look)
                    .foregroundStyle(activity.asks ? palette.red : palette.foreground)
                if !activity.more.isEmpty { Count(text: activity.more) }
                Spacer(minLength: 4)
                trailing
            }
        }
        .padding(16)
        .overlay(alignment: .leading) {
            if activity.asks { Rectangle().fill(palette.red).frame(width: 3).padding(.vertical, 18) }
        }
        .surface(look, radius: 26)
    }

    @ViewBuilder private var trailing: some View {
        if activity.running {
            StopButton(thread: activity.thread)
        } else {
            Image(systemName: "chevron.right").foregroundStyle(look.palette.mute)
        }
    }

    public init(_ activity: SurfaceActivity) { self.activity = activity }
}
