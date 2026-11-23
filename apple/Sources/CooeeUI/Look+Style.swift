import Blocks
import CooeeCore
import SwiftUI

extension Look {
    internal var type: TypeDesign { style.type }
    internal var glass: Bool { style.surface == .glass }
    internal var outline: Bool { style.fills == .outline }
    internal var textMarks: Bool { style.marks == .text }
    internal var quiet: Bool { style.sections == .quiet }
    internal var figures: Figures { style.figures }
    internal var stage: CooeeCore.Stage? { style.stage }
    internal var square: Bool { style.radius == 0 }
    internal var blocksStyle: Blocks.Style { (try? JSONValue(style).decode(Blocks.Style.self)) ?? .bundled }
    internal var pill: AnyShape { square ? AnyShape(Rectangle()) : AnyShape(Capsule()) }

    internal func label(_ role: LabelRole, _ text: String) -> String { style.labels[role]?.apply(text) ?? text }

    internal func radius(_ value: CGFloat, least: CGFloat = 0) -> CGFloat { max(value * style.radius, least) }
}

extension View {
    internal func ui(_ size: CGFloat, _ weight: Font.Weight = .regular, _ look: Look) -> some View {
        font(.system(size: size, weight: weight, design: look.type == .mono ? .monospaced : .default))
    }

    internal func surface(_ look: Look, radius: CGFloat = 22, interactive: Bool = false) -> some View {
        modifier(Surface(look: look, radius: radius, interactive: interactive))
    }

    @ViewBuilder internal func chip(_ look: Look) -> some View {
        if look.outline { self } else { surface(look, radius: 99, interactive: true) }
    }

    @ViewBuilder internal func chip(
        _ look: Look, fill: Color, edge: Color, radius: CGFloat? = nil, paper: Color = .clear, dash: [CGFloat] = []
    ) -> some View {
        if look.outline {
            background(paper).overlay(Rectangle().strokeBorder(edge, style: StrokeStyle(lineWidth: 1, dash: dash)))
        } else if let radius {
            background(fill, in: .rect(cornerRadius: radius))
        } else {
            background(fill, in: .capsule)
        }
    }

    @ViewBuilder internal func accent(_ look: Look, _ tint: Color, fill: Color, radius: CGFloat, paper: Color = .clear)
        -> some View
    {
        if look.outline {
            background(paper, in: .rect(cornerRadius: radius))
                .overlay(alignment: .leading) { Rectangle().fill(tint).frame(width: 3) }
        } else {
            background(fill, in: .rect(cornerRadius: radius))
        }
    }
}

internal struct Surface: ViewModifier {
    internal let look: Look
    internal let radius: CGFloat
    internal let interactive: Bool

    internal func body(content: Content) -> some View {
        let palette = look.palette
        switch look.style.surface {
        case .dashed:
            content.background(palette.card.opacity(0.6))
                .overlay(Rectangle().strokeBorder(palette.edge, style: StrokeStyle(lineWidth: 1, dash: [3, 2])))
        case .solid:
            content.background(palette.card, in: .rect(cornerRadius: radius * 0.6))
                .overlay(RoundedRectangle(cornerRadius: radius * 0.6).strokeBorder(palette.edge, lineWidth: 1.5))
        case .glass where look.still:
            content.background(stillGlass(palette)).overlay(stillRim(palette))
        case .glass:
            content.glassEffect(interactive ? .regular.interactive() : .regular, in: .rect(cornerRadius: radius))
        }
    }

    private func stillGlass(_ palette: Palette) -> some View {
        RoundedRectangle(cornerRadius: radius)
            .fill(palette.dark ? Color.white.opacity(0.09) : Color.white.opacity(0.62))
            .shadow(color: .black.opacity(palette.dark ? 0.35 : 0.10), radius: 10, y: 4)
    }

    private func stillRim(_ palette: Palette) -> some View {
        let rim = [Color.white.opacity(palette.dark ? 0.28 : 0.9), .white.opacity(0.04)]
        return RoundedRectangle(cornerRadius: radius).strokeBorder(
            LinearGradient(colors: rim, startPoint: .topLeading, endPoint: .bottomTrailing), lineWidth: 1)
    }
}

internal struct Backdrop: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        ZStack {
            palette.background
            if look.glass {
                RadialGradient(
                    colors: [palette.lead.opacity(palette.dark ? 0.10 : 0.22), .clear], center: .topLeading,
                    startRadius: 10, endRadius: 420)
                RadialGradient(
                    colors: [palette.cyan.opacity(palette.dark ? 0.08 : 0.12), .clear], center: .bottomTrailing,
                    startRadius: 10, endRadius: 520)
            }
        }
        .ignoresSafeArea()
    }
}

internal struct Mark<Symbol: View>: View {
    @Environment(\.look) private var look
    internal let text: String
    internal var size: CGFloat = 13
    internal var weight: Font.Weight = .bold
    @ViewBuilder internal let symbol: Symbol

    internal var body: some View {
        if look.textMarks { Text(text).ui(size, weight, look) } else { symbol }
    }
}
