import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct Activity: Sendable {
    public let persona: Persona
    public let headline: String
    public let thread: String
    public let clock: String
    public let line: String
    public let foot: String
    public let more: Int

    public static let ask = Activity(
        persona: .named("res"), headline: "Researcher asks you", thread: "Kestrel filings · mini", clock: "12m",
        line: "Allow fetch on sec.gov/cgi-bin/browse-edgar for the Q3 10-Q?", foot: "Tap to open the ask", more: 2
    )

    public static let running = Activity(
        persona: .named("coder"), headline: "Coder", thread: "Export for Kestrel · studio", clock: "2:14",
        line: "$ cargo run -p export -- --filing q3\n  wrote 412 rows to kestrel-q3.csv", foot: "working · 6 tools",
        more: 2
    )

    internal var asks: Bool { persona.state == .needs }
}

internal struct LiveActivityCard: View {
    @Environment(\.look) private var look
    internal let activity: Activity

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .top, spacing: 10) {
                Avatar(persona: activity.persona, size: 40)
                VStack(alignment: .leading, spacing: 2) {
                    Text(activity.headline).ui(16, .semibold, look)
                    Text(activity.thread).ui(13, .regular, look).foregroundStyle(palette.mute)
                }
                Spacer(minLength: 4)
                Text(activity.clock).ui(15, .semibold, look).monospacedDigit()
                    .foregroundStyle(activity.asks ? palette.red : palette.cyan)
            }
            Text(activity.line).ui(13, .regular, look.with(mono: !activity.asks)).lineLimit(2)
                .foregroundStyle(activity.asks ? palette.foreground : palette.mute)
            HStack(spacing: 8) {
                StateMark(state: activity.persona.state)
                Text(activity.foot).ui(13, .medium, look)
                    .foregroundStyle(activity.asks ? palette.red : palette.foreground)
                if activity.more > 0 { Count(text: "+\(activity.more) running") }
                Spacer(minLength: 4)
                if activity.asks { Image(systemName: "chevron.right").foregroundStyle(palette.mute) } else { stop }
            }
        }
        .padding(16)
        .overlay(alignment: .leading) { rule }
        .surface(look, radius: 26)
    }

    private var stop: some View { StopButton(compact: true) }

    @ViewBuilder private var rule: some View {
        if activity.asks { Rectangle().fill(look.palette.red).frame(width: 3).padding(.vertical, 18) }
    }
}

extension Look {
    internal func with(mono: Bool) -> Look {
        mono && style == .modern ? Look(palette: palette, style: .terminal, still: still) : self
    }
}

public struct LockScreen: View {
    @Environment(\.look) private var look

    public var body: some View {
        let palette = look.palette
        ZStack {
            Wallpaper()
            VStack(spacing: 6) {
                Text(look.mono ? "(!) 3 need you" : "Thursday 12 November  ·  3 need you").ui(17, .semibold, look)
                Text("9:41").font(.system(size: 104, weight: .bold, design: look.mono ? .monospaced : .rounded))
                LockWidgets().padding(.top, 6)
                Spacer()
                LiveActivityCard(activity: .ask).padding(.horizontal, 10)
                HStack {
                    RoundButton(symbol: "flashlight.off.fill", ascii: "*", size: 50)
                    Spacer()
                    RoundButton(symbol: "camera.fill", ascii: "o", size: 50)
                }
                .padding(.horizontal, 40).padding(.top, 34).padding(.bottom, 30)
            }
            .padding(.top, 14).foregroundStyle(palette.foreground)
        }
    }

    public init() {}
}

internal struct LockWidgets: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 12) {
            VStack(spacing: 0) {
                Text("3").ui(24, .bold, look)
                Text("NEED").ui(9, .bold, look).foregroundStyle(palette.red)
            }
            .frame(width: 70, height: 70)
            .background(Circle().strokeBorder(palette.red, lineWidth: 4).padding(2))
            .background(palette.card.opacity(0.5), in: .circle)
            VStack(alignment: .leading, spacing: 2) {
                Text(look.mono ? "(!) Researcher asks" : "Researcher asks").ui(14, .bold, look)
                Text("Allow fetch on sec.gov?").ui(14, .regular, look).lineLimit(1)
                Text("Tap to open").ui(12, .medium, look).foregroundStyle(palette.mute)
            }
            .frame(width: 168, height: 70, alignment: .leading).padding(.horizontal, 12)
            .background(palette.card.opacity(0.5), in: .rect(cornerRadius: look.mono ? 0 : 18))
        }
    }
}
