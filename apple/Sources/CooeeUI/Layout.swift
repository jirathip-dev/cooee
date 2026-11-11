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
        if let pack, let art = look.art {
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

    private var html: String {
        let url = URL(string: path, relativeTo: art)?.absoluteString ?? ""
        let shown =
            path.contains("#")
            ? "<svg><use href=\"\(url)\" width=\"100%\" height=\"100%\"/></svg>" : "<img src=\"\(url)\">"
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
