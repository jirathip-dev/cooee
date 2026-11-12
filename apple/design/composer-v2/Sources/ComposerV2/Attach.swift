import CooeeCore
@testable import CooeeUI
import SwiftUI

public enum AttachTab: String, CaseIterable, Sendable {
    case camera = "Camera"
    case photos = "Photos"
    case files = "Files"
    case screen = "Screen"
    case link = "Link"

    internal var symbol: String {
        switch self {
        case .camera: "camera.fill"
        case .photos: "photo.fill"
        case .files: "doc.fill"
        case .screen: "desktopcomputer"
        case .link: "link"
        }
    }
}

internal struct Photo: Sendable {
    internal let symbol: String
    internal let tint: Tint
    internal var pick: Int?

    internal static let recent: [Photo] = [
        Photo(symbol: "camera.fill", tint: .link), Photo(symbol: "exclamationmark.triangle", tint: .amber, pick: 1),
        Photo(symbol: "rectangle.on.rectangle", tint: .cyan, pick: 2),
        Photo(symbol: "pencil.and.scribble", tint: .lead),
        Photo(symbol: "doc.text.image", tint: .purple), Photo(symbol: "chart.bar", tint: .green),
        Photo(symbol: "iphone", tint: .cyan), Photo(symbol: "receipt", tint: .amber), Photo(symbol: "map", tint: .link),
    ]
}

internal struct AttachSheet: View {
    @Environment(\.look) private var look
    internal let tab: AttachTab

    internal var body: some View {
        Sheet {
            VStack(spacing: 12) {
                header
                switch tab {
                case .screen: ScreenGrab()
                case .link: LinkPaste()
                default: grid
                }
                caption
                tabs
            }
            .padding(.horizontal, 12).padding(.bottom, 26)
        }
    }

    private var picked: Int { tab == .photos ? 2 : 1 }

    private var header: some View {
        let palette = look.palette
        return HStack {
            Text(look.mono ? "[x]" : "Cancel").ui(15, .regular, look).foregroundStyle(palette.link)
            Spacer()
            Text(title).ui(16, .semibold, look)
            Spacer()
            Text(look.mono ? "[all]" : "Albums").ui(15, .regular, look).foregroundStyle(palette.link)
                .opacity(tab == .photos ? 1 : 0)
        }
        .padding(.horizontal, 6)
    }

    private var title: String {
        switch tab {
        case .screen: look.mono ? "# coder's screen" : "Coder's Screen"
        case .link: look.mono ? "# paste a link" : "Paste a link"
        default: look.mono ? "# \(picked) picked" : "\(picked) selected"
        }
    }

    private var grid: some View {
        LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 3), count: 3), spacing: 3) {
            ForEach(Array(Photo.recent.enumerated()), id: \.offset) { index, photo in
                Tile(photo: photo, live: index == 0)
            }
        }
        .clipShape(.rect(cornerRadius: look.mono ? 0 : 14))
    }

    private var caption: some View {
        let palette = look.palette
        return HStack(alignment: .bottom, spacing: 8) {
            Text(look.mono ? "> caption_" : "Add a caption…").ui(16, .regular, look).foregroundStyle(palette.mute)
                .padding(.horizontal, 16).frame(height: 42).frame(maxWidth: .infinity, alignment: .leading)
                .surface(look, radius: 21)
            SendButton().overlay(alignment: .topTrailing) {
                Text("\(picked)").ui(11, .bold, look).foregroundStyle(palette.link).frame(width: 18, height: 18)
                    .background(palette.background, in: .circle).offset(x: 4, y: -4)
            }
        }
    }

    private var tabs: some View {
        let palette = look.palette
        return HStack {
            ForEach(AttachTab.allCases, id: \.self) { item in
                let on = item == tab
                VStack(spacing: 5) {
                    Glyph(symbol: item.symbol, ascii: "[\(item.rawValue.prefix(1).lowercased())]", size: 18)
                        .foregroundStyle(on ? (palette.dark ? Color.black : .white) : palette.foreground)
                        .frame(width: 46, height: 46)
                        .background(
                            on ? palette.link : palette.card, in: look.mono ? AnyShape(Rectangle()) : AnyShape(Circle())
                        )
                    Text(look.mono ? item.rawValue.lowercased() : item.rawValue).ui(11, on ? .semibold : .regular, look)
                        .foregroundStyle(on ? palette.link : palette.mute)
                }
                .frame(maxWidth: .infinity)
            }
        }
        .padding(.top, 4)
    }
}

internal struct Tile: View {
    @Environment(\.look) private var look
    internal let photo: Photo
    internal let live: Bool

    internal var body: some View {
        let palette = look.palette
        let tint = palette.tint(photo.tint)
        ZStack(alignment: .topTrailing) {
            Rectangle()
                .fill(
                    live
                        ? AnyShapeStyle(Color.black.opacity(0.85))
                        : AnyShapeStyle(
                            LinearGradient(
                                colors: [tint.opacity(0.55), tint.opacity(0.2)], startPoint: .top, endPoint: .bottom))
                )
                .overlay {
                    Image(systemName: photo.symbol).font(.system(size: 26)).foregroundStyle(
                        live ? .white : palette.foreground.opacity(0.75))
                }
                .aspectRatio(1, contentMode: .fit)
            if !live { mark.padding(6) }
        }
        .overlay { if photo.pick != nil { Rectangle().strokeBorder(palette.link, lineWidth: 3) } }
    }

    @ViewBuilder private var mark: some View {
        let palette = look.palette
        if let pick = photo.pick {
            Text(look.mono ? "[\(pick)]" : "\(pick)").ui(13, .bold, look).foregroundStyle(
                look.mono ? palette.link : .white
            )
            .frame(width: 24, height: 24).background(look.mono ? palette.background : palette.link, in: .circle)
        } else {
            Circle().strokeBorder(.white, lineWidth: 1.5).frame(width: 24, height: 24).shadow(radius: 1)
        }
    }
}

internal struct ScreenGrab: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 8) {
            ZStack(alignment: .topLeading) {
                LinearGradient(
                    colors: [palette.cyan.opacity(0.5), palette.purple.opacity(0.45)], startPoint: .topLeading,
                    endPoint: .bottomTrailing)
                window(x: 18, y: 26, width: 190, height: 150, title: "look.spec.ts — 3 failed", tint: palette.red)
                window(x: 150, y: 70, width: 190, height: 132, title: "Simulator · iPhone 17", tint: palette.link)
                Rectangle().fill(.black.opacity(0.25)).frame(height: 12)
            }
            .frame(height: 232).clipShape(.rect(cornerRadius: look.mono ? 0 : 14))
            HStack(spacing: 8) {
                StateMark()
                Text(look.mono ? "mini · coder's screen · now" : "mini · from Coder's Screen · just now").ui(
                    13, .regular, look
                )
                .foregroundStyle(palette.mute)
                Spacer()
                Text(look.mono ? "[retake]" : "Retake").ui(14, .semibold, look).foregroundStyle(palette.link)
            }
        }
    }

    private func window(x: CGFloat, y: CGFloat, width: CGFloat, height: CGFloat, title: String, tint: Color)
        -> some View
    {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 4) {
                ForEach([0xff5f57, 0xfebc2e, 0x28c840] as [UInt32], id: \.self) {
                    Circle().fill(Color.hex($0)).frame(width: 6, height: 6)
                }
                Text(title).font(.system(size: 8, weight: .semibold)).foregroundStyle(.black.opacity(0.7))
            }
            ForEach(0..<6, id: \.self) { line in
                Capsule().fill(line == 2 ? tint.opacity(0.7) : Color.black.opacity(0.15)).frame(
                    width: CGFloat(60 + line * 37 % 90), height: 5)
            }
        }
        .padding(8).frame(width: width, height: height, alignment: .topLeading)
        .background(Color.white.opacity(0.92), in: .rect(cornerRadius: 6)).shadow(radius: 6)
        .offset(x: x, y: y)
    }
}

internal struct StateMark: View {
    @Environment(\.look) private var look

    internal var body: some View {
        if look.mono {
            Text("[on]").ui(11, .bold, look).foregroundStyle(look.palette.green)
        } else {
            Circle().fill(look.palette.green).frame(width: 8, height: 8)
        }
    }
}

internal struct LinkPaste: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 10) {
            Text("https://github.com/jirathip-dev/cooee/actions/runs/1187▍").ui(15, .regular, look)
                .padding(.horizontal, 14).frame(height: 44).frame(maxWidth: .infinity, alignment: .leading)
                .surface(look, radius: 14)
            HStack(spacing: 10) {
                Glyph(symbol: "link", ascii: "[url]", size: 16).foregroundStyle(palette.link).frame(
                    width: 48, height: 48
                )
                .background(palette.link.opacity(0.14), in: .rect(cornerRadius: look.mono ? 0 : 10))
                VStack(alignment: .leading, spacing: 3) {
                    Text("apple · run #1187 · failed").ui(15, .semibold, look)
                    Text("github.com · from the clipboard").ui(13, .regular, look).foregroundStyle(palette.mute)
                }
            }
        }
        .frame(height: 232, alignment: .top)
    }
}
