import CoreImage
import ImageIO
import SwiftUI

internal enum PictureSource: Hashable, Sendable {
    case sprite(String)
    case data(String)

    fileprivate var key: NSString {
        switch self {
        case .sprite(let name): "sprite:\(name)" as NSString
        case .data(let src): "data:\(src.hashValue)" as NSString
        }
    }
}

internal final class Pictures: Sendable {
    internal static let shared = Pictures()
    internal static let context = CIContext()
    private nonisolated(unsafe) let kept = NSCache<NSString, CGImage>()
    private let read: @Sendable (URL) -> Data?

    internal init(read: @escaping @Sendable (URL) -> Data? = { try? Data(contentsOf: $0) }) {
        self.read = read
        kept.totalCostLimit = 64 << 20
    }

    internal func drawn(_ source: PictureSource, still: Bool) -> CGImage? {
        guard still, case .sprite(let name) = source else { return kept.object(forKey: source.key) }
        return sprite(name)
    }

    @concurrent internal func decoded(_ source: PictureSource) async -> CGImage? {
        switch source {
        case .sprite(let name): sprite(name)
        case .data(let src): picture(src)
        }
    }

    private func sprite(_ name: String) -> CGImage? {
        let key = PictureSource.sprite(name).key
        if let image = kept.object(forKey: key) { return image }
        guard let url = Bundle.module.url(forResource: name, withExtension: "png", subdirectory: "Sprites"),
            let data = read(url), let source = CGImageSourceCreateWithData(data as CFData, nil),
            let image = CGImageSourceCreateImageAtIndex(
                source, 0, [kCGImageSourceShouldCacheImmediately: true] as CFDictionary)
        else { return nil }
        return keep(image, key)
    }

    private func picture(_ src: String) -> CGImage? {
        precondition(!Thread.isMainThread, "block images decode off the main actor")
        let key = PictureSource.data(src).key
        if let image = kept.object(forKey: key) { return image }
        guard let comma = src.firstIndex(of: ","),
            let data = Data(base64Encoded: String(src[src.index(after: comma)...])),
            let image = CIImage(data: data), let drawn = Self.context.createCGImage(image, from: image.extent)
        else { return nil }
        return keep(drawn, key)
    }

    private func keep(_ image: CGImage, _ key: NSString) -> CGImage {
        kept.setObject(image, forKey: key, cost: image.bytesPerRow * image.height)
        return image
    }
}

extension EnvironmentValues {
    @Entry internal var pictures = Pictures.shared
}

internal struct Decoded<Shown: View, Waiting: View>: View {
    @Environment(\.look) private var look
    @Environment(\.pictures) private var pictures
    @State private var loaded: (source: PictureSource, image: CGImage?)?
    internal let source: PictureSource
    @ViewBuilder internal let shown: (CGImage) -> Shown
    @ViewBuilder internal let waiting: () -> Waiting

    internal var body: some View {
        if let image = pictures.drawn(source, still: look.still) ?? (loaded?.source == source ? loaded?.image : nil) {
            shown(image)
        } else {
            waiting().task(id: source) { loaded = (source, await pictures.decoded(source)) }
        }
    }
}
