import SwiftUI

public enum Style: String, CaseIterable, Sendable {
    case modern, terminal, scene
}

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

    public let background: Color
    public let card: Color
    public let edge: Color
    public let line: Color
    public let foreground: Color
    public let mute: Color
    public let lead: Color
    public let link: Color
    public let green: Color
    public let amber: Color
    public let red: Color
    public let purple: Color
    public let cyan: Color
    public let dark: Bool
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
    public let palette: Palette
    public let style: Style
    public let phone: Bool

    public init(palette: Palette, style: Style, phone: Bool) {
        self.palette = palette
        self.style = style
        self.phone = phone
    }

    internal var mono: Bool { style == .terminal }
    internal var radius: CGFloat { style == .modern ? 12 : style == .scene ? 8 : 0 }
    internal var bodySize: CGFloat { mono ? 14.5 : 16 }
    internal var codeSize: CGFloat { 13 }
    internal var column: CGFloat { phone ? 370 : 720 }

    internal func body(_ weight: Font.Weight = .regular) -> Font {
        .system(size: bodySize, weight: weight, design: mono ? .monospaced : .default)
    }

    internal func small(_ weight: Font.Weight = .regular) -> Font {
        .system(size: 12, weight: weight, design: mono ? .monospaced : .default)
    }

    internal func code(_ weight: Font.Weight = .regular) -> Font {
        .system(size: codeSize, weight: weight, design: .monospaced)
    }

    internal func fits(_ characters: Int, width: CGFloat) -> Bool {
        CGFloat(characters) * codeSize * 0.6 <= width
    }
}

private struct LookKey: EnvironmentKey {
    fileprivate static let defaultValue = Look(palette: .farmDark, style: .modern, phone: true)
}

extension EnvironmentValues {
    internal var look: Look {
        get { self[LookKey.self] }
        set { self[LookKey.self] = newValue }
    }
}

extension View {
    internal func boxed(_ look: Look) -> some View {
        modifier(Boxed(look: look))
    }

    internal func fadesRight(_ look: Look, when overflow: Bool) -> some View {
        overlay(alignment: .trailing) {
            if overflow {
                LinearGradient(colors: [look.palette.card.opacity(0), look.palette.card], startPoint: .leading,
                    endPoint: .trailing)
                    .frame(width: 32).allowsHitTesting(false)
            }
        }
    }
}

internal struct Boxed: ViewModifier {
    internal let look: Look

    internal func body(content: Content) -> some View {
        let palette = look.palette
        switch look.style {
        case .terminal:
            content.background(palette.card.opacity(0.6))
                .overlay(Rectangle().strokeBorder(palette.edge, style: StrokeStyle(lineWidth: 1, dash: [3, 2])))
        case .scene:
            content.background(palette.card, in: .rect(cornerRadius: look.radius))
                .overlay(RoundedRectangle(cornerRadius: look.radius).strokeBorder(palette.edge, lineWidth: 1.5))
                .clipShape(.rect(cornerRadius: look.radius))
        case .modern:
            content.background(palette.card, in: .rect(cornerRadius: look.radius))
                .overlay(RoundedRectangle(cornerRadius: look.radius).strokeBorder(palette.edge, lineWidth: 1))
                .clipShape(.rect(cornerRadius: look.radius))
        }
    }
}
