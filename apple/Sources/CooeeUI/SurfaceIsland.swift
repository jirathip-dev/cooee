import CooeeCore
import SwiftUI

/// The Dynamic Island, expanded (a long-press): the persona, the thread, the clock, the last line, the tools, the
/// rest and Stop; the system draws the black around it.
public struct IslandExpanded: View {
    @Environment(\.look) private var look
    private let activity: SurfaceActivity

    public var body: some View {
        let palette = Palette.farmDark
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 10) {
                FigureView(persona: activity.persona, size: 44)
                VStack(alignment: .leading, spacing: 2) {
                    Text(activity.headline).ui(16, .semibold, look)
                    Text(activity.place).ui(13, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
                }
                Spacer()
                Text(activity.clock).ui(22, .semibold, look).monospacedDigit()
                    .foregroundStyle(activity.asks ? palette.red : palette.cyan)
            }
            Text(activity.line).font(.system(size: 13, design: .monospaced)).foregroundStyle(palette.mute).lineLimit(2)
            HStack(spacing: 8) {
                StateMark(state: activity.persona.state)
                Text(activity.running ? "\(activity.tools) tools" : activity.foot).ui(13, .medium, look)
                if !activity.more.isEmpty { Count(text: activity.more) }
                Spacer()
                if activity.running { StopButton(thread: activity.thread) }
            }
        }
        .modifier(IslandLook())
    }

    public init(_ activity: SurfaceActivity) { self.activity = activity }
}

/// The compact island's leading side: the persona.
public struct IslandMark: View {
    private let persona: Persona

    public var body: some View { FigureView(persona: persona, size: 24).modifier(IslandLook()) }

    public init(_ persona: Persona) { self.persona = persona }
}

/// The compact island's trailing side: the clock, `ask` in red, or `replied`.
public struct IslandTrailing: View {
    @Environment(\.look) private var look
    private let text: String
    private let alert: Bool

    public var body: some View {
        let palette = Palette.farmDark
        Text(text).ui(14, .semibold, look).monospacedDigit().foregroundStyle(alert ? palette.red : palette.cyan)
            .modifier(IslandLook())
    }

    public init(_ activity: SurfaceActivity) {
        text = activity.asks ? "ask" : activity.persona.state == .done ? "replied" : activity.clock
        alert = activity.asks || activity.persona.state == .fail
    }
}
