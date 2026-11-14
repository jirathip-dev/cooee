import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct Device<Content: View, Island: View>: View {
    @Environment(\.look) private var look
    private let lock: Bool
    private let island: Island
    private let content: Content

    public var body: some View {
        let foreground = look.palette.foreground
        ZStack(alignment: .top) {
            content.padding(.top, 54)
            HStack {
                Text(lock ? "" : "9:41").font(.system(size: 17, weight: .semibold)).padding(.leading, 34)
                Spacer()
                HStack(spacing: 6) {
                    Image(systemName: "cellularbars")
                    Image(systemName: "wifi")
                    Image(systemName: "battery.100")
                }
                .font(.system(size: 15, weight: .semibold)).padding(.trailing, 30)
            }
            .foregroundStyle(foreground).frame(height: 54)
            island.padding(.top, 11)
            VStack {
                Spacer()
                Capsule().fill(foreground.opacity(0.85)).frame(width: 140, height: 5).padding(.bottom, 8)
            }
        }
    }

    public init(lock: Bool = false, @ViewBuilder island: () -> Island, @ViewBuilder content: () -> Content) {
        self.lock = lock
        self.island = island()
        self.content = content()
    }
}

extension Device where Island == BareIsland {
    public init(lock: Bool = false, @ViewBuilder content: () -> Content) {
        self.init(lock: lock, island: { BareIsland() }, content: content)
    }
}

public struct BareIsland: View {
    public var body: some View { Capsule().fill(.black).frame(width: 124, height: 36) }
}

internal struct Wallpaper: View {
    @Environment(\.look) private var look
    internal var dim = false

    internal var body: some View {
        let palette = look.palette
        ZStack {
            palette.background
            if look.style != .terminal {
                LinearGradient(
                    colors: [palette.cyan.opacity(0.35), palette.lead.opacity(0.30), palette.purple.opacity(0.30)],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )
            }
            if dim { Color.black.opacity(palette.dark ? 0.45 : 0.25) }
        }
        .padding(.top, -54)
    }
}

internal struct Kicker: View {
    @Environment(\.look) private var look
    internal let text: String
    internal var tint: Color?

    internal var body: some View {
        Text(look.mono ? "# \(text.lowercased())" : text.uppercased()).ui(11, .bold, look)
            .foregroundStyle(tint ?? look.palette.mute).tracking(look.mono ? 0 : 0.6)
    }
}

internal struct StopButton: View {
    @Environment(\.look) private var look
    internal var compact = false

    internal var body: some View {
        let red = look.palette.red
        HStack(spacing: 5) {
            if !look.mono { Image(systemName: "stop.fill").font(.system(size: compact ? 10 : 12)) }
            Text(look.mono ? "[■ Stop]" : "Stop")
        }
        .fixedSize()
        .ui(compact ? 13 : 15, .semibold, look).foregroundStyle(red)
        .padding(.horizontal, compact ? 10 : 16).frame(height: compact ? 28 : 36)
        .background(red.opacity(look.mono ? 0 : 0.16), in: look.mono ? AnyShape(Rectangle()) : AnyShape(Capsule()))
    }
}

internal struct Count: View {
    @Environment(\.look) private var look
    internal let text: String

    internal var body: some View {
        Text(look.mono ? "[\(text)]" : text).ui(12, .semibold, look).foregroundStyle(look.palette.mute)
            .padding(.horizontal, 8).frame(height: 22)
            .background(look.mono ? Color.clear : look.palette.edge.opacity(0.35), in: .capsule)
    }
}

internal struct StateMark: View {
    @Environment(\.look) private var look
    internal let state: LaneState

    internal var body: some View {
        let color = tone(look.palette)
        if look.mono {
            Text(state.figure).ui(12, .bold, look).foregroundStyle(color)
        } else {
            Circle().fill(color).frame(width: 8, height: 8)
        }
    }

    private func tone(_ palette: Palette) -> Color {
        switch state {
        case .needs, .fail: palette.red
        case .working: palette.cyan
        case .done, .merged: palette.green
        case .idle: palette.mute
        }
    }
}

extension Persona {
    internal static func named(_ id: String, _ state: LaneState? = nil) -> Persona {
        let persona = Fixtures.persona(id)
        return Persona(api: persona.api, look: persona.look, state: state ?? persona.state)
    }
}
