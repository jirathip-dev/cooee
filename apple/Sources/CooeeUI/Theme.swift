import CooeeCore
import SwiftUI

public struct Palette: Sendable {
    public static let farmDark = Palette(
        background: .hex(0x0f1a13), card: .hex(0x1d1a12), edge: .hex(0x3b3a27), line: .hex(0x26301f),
        foreground: .hex(0xece4cc), mute: .hex(0xa3a68a), lead: .hex(0xecdba6), link: .hex(0x7cc4f0),
        green: .hex(0x7fd06a), amber: .hex(0xf29b4b), red: .hex(0xff7d6e), purple: .hex(0xc7a4ff),
        cyan: .hex(0x5fd0c4), dark: true
    )
    public static let farmLight = Palette(
        background: .hex(0xf5efda), card: .hex(0xebe2c4), edge: .hex(0xcbbd90), line: .hex(0xe0d6b4),
        foreground: .hex(0x29251a), mute: .hex(0x5c573f), lead: .hex(0x6b3410), link: .hex(0x1f639b),
        green: .hex(0x2a6d1c), amber: .hex(0x7a5f00), red: .hex(0xb81d3c), purple: .hex(0x7a4aa8),
        cyan: .hex(0x1c6e70), dark: false
    )

    public var background: Color
    public var card: Color
    public var edge: Color
    public var line: Color
    public var foreground: Color
    public var mute: Color
    public var lead: Color
    public var link: Color
    public var green: Color
    public var amber: Color
    public var red: Color
    public var purple: Color
    public var cyan: Color
    public var dark: Bool

    public static func farm(_ scheme: ColorScheme) -> Palette { scheme == .dark ? farmDark : farmLight }

    public func tint(_ tint: Tint) -> Color {
        switch tint {
        case .lead: lead
        case .link: link
        case .cyan: cyan
        case .purple: purple
        case .green: green
        case .amber: amber
        }
    }
}

extension Color {
    internal static func hex(_ value: UInt32) -> Color {
        Color(
            red: Double((value >> 16) & 0xff) / 255,
            green: Double((value >> 8) & 0xff) / 255,
            blue: Double(value & 0xff) / 255
        )
    }
}

public struct Look: Sendable {
    public var palette: Palette
    public var style: LookStyle
    public var still: Bool

    internal var mono: Bool { style == .terminal }

    public init(palette: Palette = .farmDark, style: LookStyle = .modern, still: Bool = false) {
        self.palette = palette
        self.style = style
        self.still = still
    }
}

private struct LookKey: EnvironmentKey {
    fileprivate static let defaultValue = Look()
}

extension EnvironmentValues {
    public var look: Look {
        get { self[LookKey.self] }
        set { self[LookKey.self] = newValue }
    }
}

extension View {
    internal func ui(_ size: CGFloat, _ weight: Font.Weight = .regular, _ look: Look) -> some View {
        font(.system(size: size, weight: weight, design: look.mono ? .monospaced : .default))
    }

    internal func surface(_ look: Look, radius: CGFloat = 22, interactive: Bool = false) -> some View {
        modifier(Surface(look: look, radius: radius, interactive: interactive))
    }
}

internal struct Surface: ViewModifier {
    internal let look: Look
    internal let radius: CGFloat
    internal let interactive: Bool

    internal func body(content: Content) -> some View {
        let palette = look.palette
        switch look.style {
        case .terminal:
            content.background(palette.card.opacity(0.6))
                .overlay(Rectangle().strokeBorder(palette.edge, style: StrokeStyle(lineWidth: 1, dash: [3, 2])))
        case .scene:
            content.background(palette.card, in: .rect(cornerRadius: radius * 0.6))
                .overlay(RoundedRectangle(cornerRadius: radius * 0.6).strokeBorder(palette.edge, lineWidth: 1.5))
        case .modern where look.still:
            content.background(stillGlass(palette)).overlay(stillRim(palette))
        case .modern:
            content.glassEffect(interactive ? .regular.interactive() : .regular, in: .rect(cornerRadius: radius))
        }
    }

    private func stillGlass(_ palette: Palette) -> some View {
        RoundedRectangle(cornerRadius: radius)
            .fill(palette.dark ? Color.white.opacity(0.09) : Color.white.opacity(0.62))
            .shadow(color: .black.opacity(palette.dark ? 0.35 : 0.10), radius: 10, y: 4)
    }

    private func stillRim(_ palette: Palette) -> some View {
        RoundedRectangle(cornerRadius: radius).strokeBorder(
            LinearGradient(
                colors: [.white.opacity(palette.dark ? 0.28 : 0.9), .white.opacity(0.04)],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            ),
            lineWidth: 1
        )
    }
}

internal struct Backdrop: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        ZStack {
            palette.background
            if look.style == .modern {
                RadialGradient(
                    colors: [palette.lead.opacity(palette.dark ? 0.10 : 0.22), .clear],
                    center: .topLeading,
                    startRadius: 10,
                    endRadius: 420
                )
                RadialGradient(
                    colors: [palette.cyan.opacity(palette.dark ? 0.08 : 0.12), .clear],
                    center: .bottomTrailing,
                    startRadius: 10,
                    endRadius: 520
                )
            }
        }
        .ignoresSafeArea()
    }
}
