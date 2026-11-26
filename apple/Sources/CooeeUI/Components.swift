import CooeeCore
import SwiftUI

internal struct FigureView: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal var size: CGFloat = 44
    internal var frame = 0

    internal var body: some View {
        if persona.id.isEmpty && look.figures != .ascii {
            NeutralAvatar(persona: persona, size: size)
        } else {
            styled
        }
    }

    @ViewBuilder private var styled: some View {
        switch look.figures {
        case .avatar:
            if let photo = AvatarPicture.image(persona.api.avatar) {
                Image(decorative: photo, scale: 1).resizable().scaledToFill().frame(width: size, height: size)
                    .clipShape(Circle())
            } else {
                BirdView(persona: persona, size: size)
            }
        case .ascii: FaceView(persona: persona, size: size)
        case .sprite: StateSprite(persona: persona, frame: frame, size: size)
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
            Text(
                [persona.api.model, effort ? persona.api.effort : ""].filter { !$0.isEmpty }.map { "· \($0)" }
                    .joined(separator: " "))
            Image(systemName: "chevron.down").font(.system(size: 8, weight: .bold))
        }
        .ui(12, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
        .padding(.horizontal, 9).padding(.vertical, 4).chip(look, fill: palette.edge.opacity(0.35), edge: palette.edge)
    }
}

internal struct RoundButton: View {
    @Environment(\.look) private var look
    internal let symbol: String
    internal var ascii = ""
    internal var size: CGFloat = 40

    internal var body: some View {
        Mark(text: "[\(ascii)]", size: 15) {
            Image(systemName: symbol).font(.system(size: size * 0.42, weight: .semibold))
        }
        .fixedSize().foregroundStyle(look.palette.foreground).frame(minWidth: size, minHeight: size)
        .chip(look)
    }
}

internal struct Badge: View {
    @Environment(\.look) private var look
    internal let count: Int
    internal var tint: Color?

    internal var body: some View {
        Text("\(count)").ui(12, .bold, look).foregroundStyle(look.palette.dark ? .black : .white)
            .padding(.horizontal, 6).frame(minWidth: 20, minHeight: 20)
            .background(tint ?? look.palette.link, in: look.pill)
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
        Text(look.label(.button, title)).ui(15, primary ? .semibold : .regular, look)
            .foregroundStyle(primary ? (palette.dark ? Color.black : .white) : tint)
            .padding(.horizontal, 14).frame(height: 36).frame(maxWidth: .infinity)
            .background(primary ? tint : tint.opacity(look.outline ? 0 : 0.12), in: look.pill)
            .accessibilityLabel(title)
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
