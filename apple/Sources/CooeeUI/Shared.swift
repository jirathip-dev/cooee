import CooeeCore
import SwiftUI
import WebKit

#if canImport(UIKit)
    import UIKit
#else
    import AppKit
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
        } else {
            Decoded(source: .sprite(name)) {
                Image(decorative: $0, scale: 1).resizable().interpolation(.high).scaledToFit()
            } waiting: {
                Color.clear
            }
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
    internal var top: (() -> Void)?
    @ViewBuilder internal var content: Content
    @State private var pinned = true
    @State private var high = false

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
            .defaultScrollAnchor(end ? .bottom : .top, for: .initialOffset)
            .defaultScrollAnchor(end ? .bottom : .top, for: .alignment)
            .defaultScrollAnchor(end && (pinned || high) ? .bottom : .top, for: .sizeChanges)
            .onScrollGeometryChange(for: Bool.self) {
                $0.visibleRect.maxY >= $0.contentSize.height - 24
            } action: {
                pinned = $1
            }
            .onScrollGeometryChange(for: CGFloat?.self) {
                $0.contentOffset.y < $0.containerSize.height ? $0.contentSize.height : nil
            } action: {
                high = $1 != nil && top != nil
                if high { top?() }
            }
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
    /// Modern, the one built-in style (`Default/style.json`): drawn until a pack's tokens are kept.
    public static let bundled: Style = {
        // SAFETY: Default/style.json is copied into this module's bundle by Package.swift
        let url = Bundle.module.url(forResource: "style", withExtension: "json", subdirectory: "Default")!
        // SAFETY: DefaultLayoutTests decodes the bundled style, so a bad edit fails the tests first
        return try! JSONDecoder().decode(Style.self, from: Data(contentsOf: url))
    }()
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
