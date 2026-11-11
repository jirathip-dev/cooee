import CooeeCore
import SwiftUI

internal struct Avatar: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal var size: CGFloat = 44

    internal var body: some View {
        let palette = look.palette
        let tint = palette.tint(persona.look.tint)
        switch look.style {
        case .modern:
            Circle()
                .fill(
                    LinearGradient(
                        colors: [tint.opacity(0.95), tint.opacity(0.6)], startPoint: .top, endPoint: .bottom)
                )
                .overlay(
                    Image(systemName: persona.look.symbol).font(.system(size: size * 0.40, weight: .semibold))
                        .foregroundStyle(palette.dark ? Color.black.opacity(0.75) : .white)
                )
                .frame(width: size, height: size)
        case .terminal:
            Text(persona.figure).font(.system(size: size * 0.30, weight: .bold, design: .monospaced))
                .foregroundStyle(tint).frame(width: size, height: size)
                .overlay(Rectangle().strokeBorder(tint.opacity(0.7), lineWidth: 1))
        case .scene:
            Sprite(name: "horse-\(persona.look.coat)-\(persona.pose)").frame(width: size * 1.32, height: size)
        }
    }
}

internal struct AgentChip: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal var effort = false

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 4) {
            Text(persona.api.agent).fontWeight(.semibold)
            Text("· \(persona.api.model)\(effort ? " · \(persona.api.effort)" : "")")
            Image(systemName: "chevron.down").font(.system(size: 8, weight: .bold))
        }
        .ui(12, .regular, look).foregroundStyle(palette.mute).lineLimit(1).fixedSize()
        .padding(.horizontal, 9).padding(.vertical, 4)
        .background(look.mono ? Color.clear : palette.edge.opacity(0.35), in: .capsule)
        .overlay {
            if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) }
        }
    }
}

internal struct RoundButton: View {
    @Environment(\.look) private var look
    internal let symbol: String
    internal var ascii = ""
    internal var size: CGFloat = 40

    internal var body: some View {
        Group {
            if look.mono {
                Text("[\(ascii)]").ui(15, .bold, look)
            } else {
                Image(systemName: symbol).font(.system(size: size * 0.42, weight: .semibold))
            }
        }
        .foregroundStyle(look.palette.foreground).frame(width: size, height: size)
        .modifier(RoundSurface(look: look))
    }
}

internal struct RoundSurface: ViewModifier {
    internal let look: Look

    internal func body(content: Content) -> some View {
        if look.mono { content } else { content.surface(look, radius: 99, interactive: true) }
    }
}

internal struct Badge: View {
    @Environment(\.look) private var look
    internal let count: Int
    internal var tint: Color?

    internal var body: some View {
        Text("\(count)").ui(12, .bold, look).foregroundStyle(look.palette.dark ? .black : .white)
            .padding(.horizontal, 6).frame(minWidth: 20, minHeight: 20)
            .background(tint ?? look.palette.link, in: look.mono ? AnyShape(Rectangle()) : AnyShape(Capsule()))
    }
}

internal struct CardButton: View {
    @Environment(\.look) private var look
    internal let title: String
    internal var primary = false
    internal var tint: Color?

    internal var body: some View {
        let palette = look.palette
        let tint = tint ?? palette.link
        Text(look.mono ? "[ \(title) ]" : title).ui(15, primary ? .semibold : .regular, look)
            .foregroundStyle(primary ? (palette.dark ? Color.black : .white) : tint)
            .padding(.horizontal, 14).frame(height: 36).frame(maxWidth: .infinity)
            .background(
                primary ? tint : tint.opacity(look.mono ? 0 : 0.12),
                in: look.mono ? AnyShape(Rectangle()) : AnyShape(Capsule())
            )
    }
}

public struct PhoneChrome<Content: View>: View {
    @Environment(\.look) private var look
    private let content: Content

    public var body: some View {
        let foreground = look.palette.foreground
        ZStack(alignment: .top) {
            content.padding(.top, 54)
            HStack {
                Text("9:41").font(.system(size: 17, weight: .semibold)).padding(.leading, 34)
                Spacer()
                HStack(spacing: 6) {
                    Image(systemName: "cellularbars")
                    Image(systemName: "wifi")
                    Image(systemName: "battery.100")
                }
                .font(.system(size: 15, weight: .semibold)).padding(.trailing, 30)
            }
            .foregroundStyle(foreground).frame(height: 54)
            Capsule().fill(.black).frame(width: 124, height: 36).padding(.top, 11)
            VStack {
                Spacer()
                Capsule().fill(foreground.opacity(0.85)).frame(width: 140, height: 5).padding(.bottom, 8)
            }
        }
    }

    public init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }
}
