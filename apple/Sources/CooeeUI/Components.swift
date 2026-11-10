import SwiftUI

/// A persona's avatar in the look's style: Modern an SF Symbol on its tint, Terminal its ASCII figure in brackets,
/// Scene its horse in its coat, acting its state.
struct Avatar: View {
    @Environment(\.look) var look
    let persona: Persona
    var size: CGFloat = 44
    var body: some View {
        let p = look.palette, tint = p.tint(persona.tint)
        switch look.style {
        case .modern:
            Circle().fill(LinearGradient(colors: [tint.opacity(0.95), tint.opacity(0.6)], startPoint: .top, endPoint: .bottom))
                .overlay(Image(systemName: persona.symbol).font(.system(size: size * 0.40, weight: .semibold))
                    .foregroundStyle(p.dark ? Color.black.opacity(0.75) : .white))
                .frame(width: size, height: size)
        case .terminal:
            Text(persona.figure).font(.system(size: size * 0.30, weight: .bold, design: .monospaced))
                .foregroundStyle(tint).frame(width: size, height: size)
                .overlay(Rectangle().strokeBorder(tint.opacity(0.7), lineWidth: 1))
        case .scene:
            Sprite(name: "horse-\(persona.coat)-\(persona.pose)").frame(width: size * 1.32, height: size)
        }
    }
}

/// `Claude Code · opus 5.5 ▾`: the persona's agent and model (q888/q902).
struct AgentChip: View {
    @Environment(\.look) var look
    let persona: Persona
    var effort = false
    var body: some View {
        let p = look.palette
        HStack(spacing: 4) {
            Text(persona.agent).fontWeight(.semibold)
            Text("· \(persona.model)\(effort ? " · \(persona.effort)" : "")")
            Image(systemName: "chevron.down").font(.system(size: 8, weight: .bold))
        }
        .ui(12, .regular, look).foregroundStyle(p.mute).lineLimit(1).fixedSize()
        .padding(.horizontal, 9).padding(.vertical, 4)
        .background(look.mono ? Color.clear : p.edge.opacity(0.35), in: .capsule)
        .overlay { if look.mono { Rectangle().strokeBorder(p.edge, lineWidth: 1) } }
    }
}

/// A round glass button with an SF Symbol (Terminal: a bracketed glyph).
struct RoundButton: View {
    @Environment(\.look) var look
    let symbol: String
    var ascii = ""
    var size: CGFloat = 40
    var body: some View {
        let p = look.palette
        Group {
            if look.mono {
                Text("[\(ascii)]").ui(15, .bold, look)
            } else {
                Image(systemName: symbol).font(.system(size: size * 0.42, weight: .semibold))
            }
        }
        .foregroundStyle(p.fg).frame(width: size, height: size)
        .modifier(RoundSurface(look: look))
    }
}

struct RoundSurface: ViewModifier {
    let look: Look
    func body(content: Content) -> some View {
        if look.mono { content } else { content.surface(look, radius: 99, interactive: true) }
    }
}

/// The unread badge.
struct Badge: View {
    @Environment(\.look) var look
    let n: Int
    var body: some View {
        Text("\(n)").ui(12, .bold, look).foregroundStyle(look.palette.dark ? .black : .white)
            .padding(.horizontal, 6).frame(minWidth: 20, minHeight: 20)
            .background(look.palette.link, in: look.mono ? AnyShape(Rectangle()) : AnyShape(Capsule()))
    }
}

/// A filled or plain action button inside a card.
struct CardButton: View {
    @Environment(\.look) var look
    let title: String
    var primary = false
    var tint: Color? = nil
    var body: some View {
        let p = look.palette, t = tint ?? p.link
        Text(look.mono ? "[ \(title) ]" : title).ui(15, primary ? .semibold : .regular, look)
            .foregroundStyle(primary ? (p.dark ? Color.black : .white) : t)
            .padding(.horizontal, 14).frame(height: 36).frame(maxWidth: .infinity)
            .background(primary ? t : t.opacity(look.mono ? 0 : 0.12),
                        in: look.mono ? AnyShape(Rectangle()) : AnyShape(Capsule()))
    }
}

/// The device frame for a still: the status bar and home indicator, so a 402 × 874 PNG reads as an iPhone 17.
public struct PhoneChrome<Content: View>: View {
    @Environment(\.look) var look
    let content: Content
    public init(@ViewBuilder content: () -> Content) { self.content = content() }
    public var body: some View {
        let fg = look.palette.fg
        ZStack(alignment: .top) {
            content.padding(.top, 54)
            HStack {
                Text("9:41").font(.system(size: 17, weight: .semibold)).padding(.leading, 34)
                Spacer()
                HStack(spacing: 6) {
                    Image(systemName: "cellularbars"); Image(systemName: "wifi"); Image(systemName: "battery.100")
                }.font(.system(size: 15, weight: .semibold)).padding(.trailing, 30)
            }
            .foregroundStyle(fg).frame(height: 54)
            Capsule().fill(.black).frame(width: 124, height: 36).padding(.top, 11)
            VStack { Spacer(); Capsule().fill(fg.opacity(0.85)).frame(width: 140, height: 5).padding(.bottom, 8) }
        }
    }
}
