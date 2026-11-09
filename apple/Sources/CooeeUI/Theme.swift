import SwiftUI

/// The three styles (q947): Modern is Apple-native (Liquid Glass, SF Symbols), Terminal is monospace with the ASCII
/// figures, Scene draws the personas as the theme's sprites. Each takes its colours from the scheme.
public enum LookStyle: String, CaseIterable, Sendable { case modern, terminal, scene }

/// A scheme's tokens, as `web/schemes/*.json` names them.
public struct Palette: Sendable {
    public var bg, card, edge, line, fg, mute, l, link, g, a, r, p, c: Color
    public var dark: Bool

    /// `web/schemes/farm.json`, the default scheme (q947: the farm by default).
    public static let farmDark = Palette(
        bg: .hex(0x0f1a13), card: .hex(0x1d1a12), edge: .hex(0x3b3a27), line: .hex(0x26301f), fg: .hex(0xece4cc),
        mute: .hex(0xa3a68a), l: .hex(0xecdba6), link: .hex(0x7cc4f0), g: .hex(0x7fd06a), a: .hex(0xf29b4b),
        r: .hex(0xff7d6e), p: .hex(0xc7a4ff), c: .hex(0x5fd0c4), dark: true)
    /// `web/schemes/farm-light.json`.
    public static let farmLight = Palette(
        bg: .hex(0xf5efda), card: .hex(0xebe2c4), edge: .hex(0xcbbd90), line: .hex(0xe0d6b4), fg: .hex(0x29251a),
        mute: .hex(0x5c573f), l: .hex(0x6b3410), link: .hex(0x1f639b), g: .hex(0x2a6d1c), a: .hex(0x7a5f00),
        r: .hex(0xb81d3c), p: .hex(0x7a4aa8), c: .hex(0x1c6e70), dark: false)

    public static func farm(_ scheme: ColorScheme) -> Palette { scheme == .dark ? farmDark : farmLight }

    /// A persona's accent, by the token the fixtures name.
    public func tint(_ t: Tint) -> Color {
        switch t { case .l: l; case .link: link; case .c: c; case .p: p; case .g: g; case .a: a }
    }
}

public enum Tint: Sendable { case l, link, c, p, g, a }

extension Color {
    static func hex(_ v: UInt32) -> Color {
        Color(red: Double((v >> 16) & 0xff) / 255, green: Double((v >> 8) & 0xff) / 255, blue: Double(v & 0xff) / 255)
    }
}

/// The look the views read: palette, style, and whether this is a still for the design (ImageRenderer draws no
/// platform views, so a still lays scroll views and fields out as plain stacks and draws glass as a stand-in).
public struct Look: Sendable {
    public var palette: Palette
    public var style: LookStyle
    public var still: Bool
    public init(palette: Palette = .farmDark, style: LookStyle = .modern, still: Bool = false) {
        self.palette = palette; self.style = style; self.still = still
    }
    var mono: Bool { style == .terminal }
}

private struct LookKey: EnvironmentKey { static let defaultValue = Look() }

extension EnvironmentValues {
    /// The look every view reads (an `EnvironmentKey`, not `@Entry`: the package builds without Xcode's macros too).
    public var look: Look {
        get { self[LookKey.self] }
        set { self[LookKey.self] = newValue }
    }
}

extension View {
    /// The body font: SF in Modern and Scene, SF Mono in Terminal.
    func ui(_ size: CGFloat, _ weight: Font.Weight = .regular, _ look: Look) -> some View {
        font(.system(size: size, weight: weight, design: look.mono ? .monospaced : .default))
    }

    /// Liquid Glass (Modern), a ruled box (Terminal), or the card colour (Scene). A still draws glass as a tinted,
    /// rimmed fill, since ImageRenderer leaves `glassEffect` out.
    func surface(_ look: Look, radius: CGFloat = 22, interactive: Bool = false) -> some View {
        modifier(Surface(look: look, radius: radius, interactive: interactive))
    }
}

struct Surface: ViewModifier {
    let look: Look, radius: CGFloat, interactive: Bool
    func body(content: Content) -> some View {
        let p = look.palette
        switch look.style {
        case .terminal:
            content.background(p.card.opacity(0.6))
                .overlay(Rectangle().strokeBorder(p.edge, style: StrokeStyle(lineWidth: 1, dash: [3, 2])))
        case .scene:
            content.background(p.card, in: .rect(cornerRadius: radius * 0.6))
                .overlay(RoundedRectangle(cornerRadius: radius * 0.6).strokeBorder(p.edge, lineWidth: 1.5))
        case .modern:
            if look.still {
                content
                    .background(
                        RoundedRectangle(cornerRadius: radius)
                            .fill(p.dark ? Color.white.opacity(0.09) : Color.white.opacity(0.62))
                            .shadow(color: .black.opacity(p.dark ? 0.35 : 0.10), radius: 10, y: 4))
                    .overlay(
                        RoundedRectangle(cornerRadius: radius).strokeBorder(
                            LinearGradient(colors: [.white.opacity(p.dark ? 0.28 : 0.9), .white.opacity(0.04)],
                                           startPoint: .topLeading, endPoint: .bottomTrailing), lineWidth: 1))
            } else {
                content.glassEffect(interactive ? .regular.interactive() : .regular, in: .rect(cornerRadius: radius))
            }
        }
    }
}

/// A screen's backdrop: the scheme's bg; Modern adds a soft wash so the glass has something to refract.
struct Backdrop: View {
    @Environment(\.look) var look
    var body: some View {
        let p = look.palette
        ZStack {
            p.bg
            if look.style == .modern {
                RadialGradient(colors: [p.l.opacity(p.dark ? 0.10 : 0.22), .clear], center: .topLeading,
                               startRadius: 10, endRadius: 420)
                RadialGradient(colors: [p.c.opacity(p.dark ? 0.08 : 0.12), .clear], center: .bottomTrailing,
                               startRadius: 10, endRadius: 520)
            }
        }
        .ignoresSafeArea()
    }
}

/// A sprite PNG rasterised from the farm pack (apple/design/sprites.mjs).
struct Sprite: View {
    let name: String
    var body: some View {
        if let url = Bundle.module.url(forResource: name, withExtension: "png", subdirectory: "Sprites"),
           let img = PlatformImage(contentsOfFile: url.path) {
            #if canImport(UIKit)
            Image(uiImage: img).resizable().interpolation(.high).scaledToFit()
            #else
            Image(nsImage: img).resizable().interpolation(.high).scaledToFit()
            #endif
        } else {
            Color.clear
        }
    }
}

#if canImport(UIKit)
import UIKit
typealias PlatformImage = UIImage
#else
import AppKit
typealias PlatformImage = NSImage
#endif

/// A scroll view in the app; in a still, the content laid over the space it has (top, or for a chat its end, as a
/// chat scrolled to its last item), clipped, so a tall list never pushes the header or composer out.
struct Scroller<Content: View>: View {
    @Environment(\.look) var look
    var axis: Axis.Set = .vertical
    var end = false
    @ViewBuilder var content: Content
    var body: some View {
        if look.still {
            let a: Alignment = axis == .horizontal ? .leading : (end ? .bottom : .top)
            Color.clear
                .overlay(alignment: a) {
                    if axis == .horizontal {
                        HStack(spacing: 0) { content }.fixedSize()
                    } else {
                        VStack(spacing: 0) { content }.fixedSize(horizontal: false, vertical: true)
                    }
                }
                .clipped()
        } else if axis == .horizontal {
            ScrollView(.horizontal, showsIndicators: false) { HStack(spacing: 0) { content } }
        } else {
            ScrollView {
                VStack(spacing: 0) { content }
            }
            .defaultScrollAnchor(end ? .bottom : .top)
        }
    }
}
