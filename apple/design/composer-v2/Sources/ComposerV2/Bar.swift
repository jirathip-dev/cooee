import CooeeCore
@testable import CooeeUI
import SwiftUI

internal struct ComposerBar: View {
    @Environment(\.look) private var look
    internal let state: BarState
    internal var name = "Coder"
    internal var wide = false

    internal var body: some View {
        switch state {
        case .recording: RecordingBar()
        case .locked: LockedBar()
        default:
            HStack(alignment: .bottom, spacing: look.mono ? 4 : 8) {
                if look.mono {
                    Text("[/]").ui(15, .bold, look).foregroundStyle(look.palette.foreground).frame(height: 42)
                } else {
                    RoundButton(symbol: "line.3.horizontal", ascii: "/", size: 42)
                }
                field
                right
            }
        }
    }

    private var text: String {
        switch state {
        case .typing: Story.typed
        case .slash: "/m"
        default: ""
        }
    }

    private var field: some View {
        let palette = look.palette
        return HStack(alignment: .firstTextBaseline, spacing: 0) {
            if look.mono { Text("> ").ui(16, .bold, look).foregroundStyle(palette.lead) }
            if text.isEmpty {
                Text(look.mono ? "_" : "Message \(name)").ui(16, .regular, look)
                    .foregroundStyle(look.mono ? palette.lead : palette.mute)
            } else {
                (Text(text) + Text(look.mono ? "_" : "▍").foregroundStyle(palette.link)).ui(16, .regular, look)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, look.mono ? 8 : 16).padding(.vertical, 10)
        .frame(minHeight: 42)
        .surface(look, radius: 21)
    }

    @ViewBuilder private var right: some View {
        let palette = look.palette
        if text.isEmpty {
            if look.mono {
                Text("[mic]").ui(15, .bold, look).foregroundStyle(palette.foreground).frame(height: 42)
            } else {
                RoundButton(symbol: "mic", ascii: "mic", size: 42)
            }
        } else if look.mono {
            Text("[send]").ui(15, .bold, look).foregroundStyle(palette.link).frame(height: 42)
        } else {
            Image(systemName: "arrow.up").font(.system(size: 17, weight: .bold))
                .foregroundStyle(palette.dark ? Color.black : .white)
                .frame(width: 42, height: 42).background(palette.link, in: .circle)
        }
    }
}

internal struct RecordingBar: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        ZStack(alignment: .bottomTrailing) {
            HStack(spacing: 10) {
                Circle().fill(palette.red).frame(width: 10, height: 10)
                Text("0:07,4").ui(16, .medium, look).monospacedDigit()
                Spacer()
                Text(look.mono ? "< slide to cancel" : "‹  Slide to cancel").ui(15, .regular, look)
                    .foregroundStyle(palette.mute)
                Spacer().frame(width: 96)
            }
            .padding(.horizontal, 16).frame(height: 42).surface(look, radius: 21)
            VStack(spacing: 14) {
                lock
                mic
            }
            .offset(x: 18, y: 22)
        }
    }

    private var lock: some View {
        let palette = look.palette
        return VStack(spacing: 6) {
            Glyph(symbol: "lock.open", ascii: "[lock]", size: 15)
            Glyph(symbol: "chevron.up", ascii: "^", size: 12).foregroundStyle(palette.mute)
        }
        .padding(.vertical, 12).frame(width: 40)
        .modifier(PillSurface(look: look))
    }

    private var mic: some View {
        let palette = look.palette
        return ZStack {
            if !look.mono { Circle().fill(palette.link.opacity(0.22)).frame(width: 118, height: 118) }
            Glyph(symbol: "mic.fill", ascii: "[rec]", size: 30)
                .foregroundStyle(palette.dark ? Color.black : .white)
                .frame(width: 86, height: 86)
                .background(
                    look.mono ? palette.red : palette.link, in: look.mono ? AnyShape(Rectangle()) : AnyShape(Circle()))
        }
        .frame(width: 118, height: 118)
    }
}

internal struct LockedBar: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .trailing, spacing: 10) {
            Glyph(symbol: "stop.fill", ascii: "[stop]", size: 14).foregroundStyle(palette.red)
                .frame(width: 40, height: 40).modifier(PillSurface(look: look))
            HStack(spacing: 10) {
                Glyph(symbol: "trash", ascii: "[x]", size: 17).foregroundStyle(palette.red)
                    .frame(width: 42, height: 42)
                HStack(spacing: 10) {
                    Circle().fill(palette.red).frame(width: 8, height: 8)
                    Text("0:12,8").ui(15, .medium, look).monospacedDigit()
                    Wave(levels: Wave.live, tint: palette.red)
                }
                .padding(.horizontal, 14).frame(height: 42).frame(maxWidth: .infinity, alignment: .leading)
                .surface(look, radius: 21)
                SendButton()
            }
        }
    }
}

internal struct SendButton: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        if look.mono {
            Text("[send]").ui(14, .bold, look).foregroundStyle(palette.link).frame(height: 42)
        } else {
            Image(systemName: "arrow.up").font(.system(size: 17, weight: .bold))
                .foregroundStyle(palette.dark ? Color.black : .white)
                .frame(width: 42, height: 42).background(palette.link, in: .circle)
        }
    }
}

internal struct PillSurface: ViewModifier {
    internal let look: Look

    internal func body(content: Content) -> some View {
        if look.mono {
            content.overlay(Rectangle().strokeBorder(look.palette.edge, lineWidth: 1))
                .background(look.palette.background)
        } else {
            content.surface(look, radius: 20)
        }
    }
}

internal struct Wave: View {
    internal static let live: [CGFloat] = [4, 9, 15, 22, 12, 6, 18, 24, 14, 8, 20, 11, 5, 16, 22, 10, 6, 13]
    internal static let note: [CGFloat] = [
        6, 10, 16, 12, 20, 8, 14, 22, 18, 9, 6, 12, 19, 24, 15, 8, 11, 17, 21, 13, 7, 10, 16, 9, 5, 12, 18, 14,
    ]
    internal let levels: [CGFloat]
    internal let tint: Color
    internal var played = 0

    internal var body: some View {
        HStack(alignment: .center, spacing: 2.5) {
            ForEach(Array(levels.enumerated()), id: \.offset) { index, level in
                Capsule().fill(index < played ? tint : tint.opacity(0.35)).frame(width: 3, height: level)
            }
        }
        .frame(height: 26)
    }
}
