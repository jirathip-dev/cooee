import CooeeCore
import SwiftUI

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
    internal let name: String

    internal var body: some View {
        if let url = Bundle.module.url(forResource: name, withExtension: "png", subdirectory: "Sprites"),
            let image = PlatformImage(contentsOfFile: url.path)
        {
            Image(platform: image).resizable().interpolation(.high).scaledToFit()
        } else {
            Color.clear
        }
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
