import CooeeCore
import SwiftUI
import WebKit

#if canImport(UIKit)
    import UIKit

    internal typealias PlatformImage = UIImage

    extension Image {
        internal init(platform: PlatformImage) { self.init(uiImage: platform) }
    }
#else
    import AppKit

    internal typealias PlatformImage = NSImage

    extension Image {
        internal init(platform: PlatformImage) { self.init(nsImage: platform) }
    }
#endif

internal struct Sprite: View {
    @Environment(\.look) private var look
    internal let name: String
    internal var pack: String?

    internal var body: some View {
        let id = pack?.split(separator: "#").last.map(String.init) ?? ""
        if let figure = look.drawn[id] {
            FigureArt(figure: figure)
        } else if let svg = look.vectors[id] {
            PackArt(art: look.art ?? URL(filePath: "/"), path: "", markup: svg)
        } else if let pack, let art = look.art {
            PackArt(art: art, path: pack)
        } else if let url = Bundle.module.url(forResource: name, withExtension: "png", subdirectory: "Sprites"),
            let image = PlatformImage(contentsOfFile: url.path)
        {
            Image(platform: image).resizable().interpolation(.high).scaledToFit()
        } else {
            Color.clear
        }
    }
}

internal struct PackArt: View {
    @State private var page = WebPage()
    internal let art: URL
    internal let path: String
    internal var markup: String?

    private var html: String {
        let url = URL(string: path, relativeTo: art)?.absoluteString ?? ""
        let shown =
            markup
            ?? (path.contains("#")
                ? "<svg><use href=\"\(url)\" width=\"100%\" height=\"100%\"/></svg>" : "<img src=\"\(url)\">")
        return "<meta name=\"viewport\" content=\"width=device-width\"><style>*{margin:0}"
            + "html,body,svg,img{width:100%;height:100%;display:block;overflow:hidden;background:none}"
            + "img{object-fit:cover;object-position:bottom}</style>\(shown)"
    }

    internal var body: some View {
        WebView(page).webViewContentBackground(.hidden).allowsHitTesting(false).accessibilityHidden(true)
            .task(id: html) { _ = page.load(html: html, baseURL: art) }
    }
}

internal struct Scroller<Content: View>: View {
    @Environment(\.look) private var look
    internal var axis: Axis.Set = .vertical
    internal var end = false
    @ViewBuilder internal var content: Content

    internal var body: some View {
        if look.still {
            Color.clear
                .overlay(alignment: axis == .horizontal ? .leading : (end ? .bottom : .top)) { still }
                .clipped()
        } else if axis == .horizontal {
            ScrollView(.horizontal, showsIndicators: false) { HStack(spacing: 0) { content } }
        } else {
            ScrollView {
                VStack(spacing: 0) { content }
            }
            .defaultScrollAnchor(end ? .bottom : .top)
            .scrollDismissesKeyboard(.interactively)
        }
    }

    @ViewBuilder private var still: some View {
        if axis == .horizontal {
            HStack(spacing: 0) { content }.fixedSize()
        } else {
            VStack(spacing: 0) { content }.fixedSize(horizontal: false, vertical: true)
        }
    }
}

internal func copy(_ text: String) {
    #if os(iOS)
        UIPasteboard.general.string = text
    #else
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    #endif
}

extension CooeeCore.Layout {
    /// The bundled default (`Default/layout.json`): today's screens, drawn before any server answers.
    public static let bundled: CooeeCore.Layout = {
        // SAFETY: Default/layout.json is copied into this module's bundle by Package.swift
        let url = Bundle.module.url(forResource: "layout", withExtension: "json", subdirectory: "Default")!
        // SAFETY: DefaultLayoutTests decodes the bundled file, so a bad edit fails the tests first
        return try! CooeeCore.Layout(json: Data(contentsOf: url), etag: "")
    }()
}

extension Style {
    private static let bundled = LookStyle.allCases.map { pick in
        let (name, folder) = pick == .modern ? ("style", "Default") : (pick.rawValue, "Default/styles")
        // SAFETY: Default/style.json and Default/styles/ are copied into this module's bundle by Package.swift
        let url = Bundle.module.url(forResource: name, withExtension: "json", subdirectory: folder)!
        // SAFETY: DefaultLayoutTests decodes every bundled style, so a bad edit fails the tests first
        return try! JSONDecoder().decode(Style.self, from: Data(contentsOf: url))
    }

    /// The bundled style for a pick: Modern's `Default/style.json`, or a pack's copy in `Default/styles/`.
    public static func bundled(_ pick: LookStyle) -> Style { bundled[LookStyle.allCases.firstIndex(of: pick) ?? 0] }
}

extension HomeScreen {
    /// The bundled home (`Default/screens/home.json`): drawn until the server's first answer, and offline.
    public static let bundled: HomeScreen = {
        // SAFETY: Default/screens/home.json is copied into this module's bundle by Package.swift
        let url = Bundle.module.url(forResource: "home", withExtension: "json", subdirectory: "Default/screens")!
        // SAFETY: DefaultLayoutTests decodes the bundled home, so a bad edit fails the tests first
        return try! JSONDecoder().decode(HomeScreen.self, from: Data(contentsOf: url))
    }()
}
