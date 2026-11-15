import CooeeCore
@testable import CooeeUI
import SwiftUI

internal struct IslandLook: ViewModifier {
    @Environment(\.look) private var look

    internal func body(content: Content) -> some View {
        content.environment(\.look, Look(palette: .farmDark, style: look.style, still: look.still))
            .environment(\.colorScheme, .dark).foregroundStyle(Palette.farmDark.foreground)
    }
}

public struct ExpandedIsland: View {
    @Environment(\.look) private var look
    private let activity = Activity.running

    public var body: some View {
        let palette = Palette.farmDark
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 10) {
                Avatar(persona: activity.persona, size: 44)
                VStack(alignment: .leading, spacing: 2) {
                    Text(activity.headline).ui(16, .semibold, look)
                    Text(activity.thread).ui(13, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
                }
                Spacer()
                Text(activity.clock).ui(22, .semibold, look).monospacedDigit().foregroundStyle(palette.cyan)
            }
            Text(activity.line).ui(13, .regular, look.with(mono: true)).foregroundStyle(palette.mute).lineLimit(2)
            HStack(spacing: 8) {
                StateMark(state: .working)
                Text("6 tools").ui(13, .medium, look)
                Count(text: "+\(activity.more) running")
                Spacer()
                StopButton(compact: true)
            }
        }
        .padding(.horizontal, 20).padding(.vertical, 16).frame(width: 384)
        .background(.black, in: .rect(cornerRadius: look.mono ? 0 : 44))
        .modifier(IslandLook())
    }

    public init() {}
}

internal struct CompactIsland: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal let trailing: String
    internal var alert = false

    internal var body: some View {
        let palette = Palette.farmDark
        HStack {
            Avatar(persona: persona, size: 24)
            Spacer()
            Text(trailing).ui(14, .semibold, look).monospacedDigit()
                .foregroundStyle(alert ? palette.red : palette.cyan)
        }
        .padding(.horizontal, 7).frame(width: 220, height: 36)
        .background(.black, in: .capsule)
        .modifier(IslandLook())
    }
}

public struct IslandScreen: View {
    @Environment(\.look) private var look

    public var body: some View {
        let palette = look.palette
        ZStack(alignment: .top) {
            Backdrop()
            OtherApp().opacity(0.35)
            VStack(alignment: .leading, spacing: 14) {
                Kicker(text: "The island, compact")
                state(
                    "Running: the persona, then the turn's clock. Long-press opens it as above; tap opens the thread.",
                    CompactIsland(persona: .named("coder"), trailing: "2:14"))
                state(
                    "An ask outranks a turn: the asking persona and `ask`. Tap opens the ask; nothing is answered here.",
                    CompactIsland(persona: .named("res"), trailing: "ask", alert: true))
                state(
                    "Ended: `replied`, for 15 minutes, then it goes.",
                    CompactIsland(persona: .named("coder", .done), trailing: "replied"))
            }
            .padding(18).surface(look, radius: 26).padding(.horizontal, 12).padding(.top, 196)
            .foregroundStyle(palette.foreground)
        }
    }

    private func state(_ note: String, _ island: CompactIsland) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            island
            Text(LocalizedStringKey(note)).ui(13, .regular, look).foregroundStyle(look.palette.mute)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    public init() {}
}

internal struct OtherApp: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 14) {
            ForEach(0..<9, id: \.self) { index in
                HStack(spacing: 12) {
                    Circle().fill(palette.edge).frame(width: 40, height: 40)
                    VStack(alignment: .leading, spacing: 6) {
                        Capsule().fill(palette.edge).frame(width: CGFloat(140 + index % 3 * 40), height: 10)
                        Capsule().fill(palette.line).frame(width: CGFloat(220 - index % 4 * 30), height: 8)
                    }
                }
            }
        }
        .padding(.horizontal, 20).padding(.top, 70).frame(maxWidth: .infinity, alignment: .leading)
    }
}
