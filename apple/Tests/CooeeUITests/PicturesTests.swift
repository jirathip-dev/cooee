@testable import CooeeUI
import Foundation
import ImageIO
import SwiftUI
import Synchronization
import Testing
import UniformTypeIdentifiers

private final class Reads: Sendable {
    fileprivate let count = Mutex(0)
}

@MainActor
internal struct PicturesTests {
    @Test
    internal func drawingTheSameSprite100TimesReadsItsFileOnce() throws {
        let reads = Reads()
        let pictures = Pictures { url in
            reads.count.withLock { $0 += 1 }
            return try? Data(contentsOf: url)
        }
        for _ in 0..<100 {
            let view = Sprite(name: "horse-bay-idle").frame(width: 40, height: 30)
                .environment(\.look, Look(still: true)).environment(\.pictures, pictures)
            let drawn = try #require(ImageRenderer(content: view).cgImage.flatMap(Pixels.init))
            #expect(stride(from: 3, to: drawn.bytes.count, by: 4).contains { drawn.bytes[$0] > 200 })
        }
        #expect(reads.count.withLock { $0 } == 1)
    }

    @Test
    internal func aTwoMegabyteBlockImageDecodesOffTheMainActor() async throws {
        let png = try Self.noise(width: 1024, height: 768)
        #expect(png.count > 2_000_000)
        let source = PictureSource.data("data:image/png;base64," + png.base64EncodedString())
        let pictures = Pictures()
        #expect(pictures.drawn(source, still: false) == nil)
        let image = try #require(await pictures.decoded(source))
        #expect(image.width == 1024 && image.height == 768)
        #expect(pictures.drawn(source, still: false) === image)
    }

    private static func noise(width: Int, height: Int) throws -> Data {
        var generator = SystemRandomNumberGenerator()
        let words = (0..<width * height / 2).map { _ in generator.next() }
        let out = NSMutableData()
        let context = try #require(
            CGContext(
                data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
                space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue))
        try words.withUnsafeBytes { context.data?.copyMemory(from: try #require($0.baseAddress), byteCount: $0.count) }
        let image = try #require(context.makeImage())
        let target = try #require(CGImageDestinationCreateWithData(out, UTType.png.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(target, image, nil)
        #expect(CGImageDestinationFinalize(target))
        return out as Data
    }
}
