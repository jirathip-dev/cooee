import AppKit
import Foundation
import Testing

internal struct Pixels: Equatable {
    internal let width: Int
    internal let height: Int
    internal let bytes: [UInt8]

    internal init?(_ image: CGImage) {
        let (columns, rows) = (image.width, image.height)
        var pixels = [UInt8](repeating: 0, count: columns * rows * 4)
        let drawn = pixels.withUnsafeMutableBytes { buffer in
            guard
                let context = CGContext(
                    data: buffer.baseAddress, width: columns, height: rows, bitsPerComponent: 8,
                    bytesPerRow: columns * 4,
                    space: CGColorSpace(name: CGColorSpace.sRGB) ?? CGColorSpaceCreateDeviceRGB(),
                    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
                )
            else { return false }
            context.draw(image, in: CGRect(x: 0, y: 0, width: columns, height: rows))
            return true
        }
        guard drawn else { return nil }
        width = columns
        height = rows
        bytes = pixels
    }

    internal func differing(from other: Pixels, by channel: Int) -> Double {
        let count = stride(from: 0, to: bytes.count, by: 4).count { start in
            (0..<4).contains { abs(Int(bytes[start + $0]) - Int(other.bytes[start + $0])) > channel }
        }
        return Double(count) / Double(width * height)
    }
}

@Suite(.serialized)
@MainActor
internal struct SnapshotTests {
    private static let channelTolerance = 16
    private static let pixelTolerance = 0.025
    private static let designs = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .appendingPathComponent("design/v1")

    @Test(arguments: Frame.all)
    internal func frameMatchesTheDesign(_ frame: Frame) throws {
        let rendered = try #require(frame.render())
        let file = Self.designs.appendingPathComponent("\(frame.name).png")
        let png = try #require(NSBitmapImageRep(cgImage: rendered).representation(using: .png, properties: [:]))
        let env = ProcessInfo.processInfo.environment
        if env["COOEE_RECORD"] == "1" { try png.write(to: file) }
        if let rendered = env["COOEE_RENDERED"] {
            try png.write(to: URL(fileURLWithPath: rendered).appendingPathComponent(file.lastPathComponent))
        }
        let design = try #require(NSImage(contentsOf: file)?.cgImage(forProposedRect: nil, context: nil, hints: nil))
        let got = try #require(Pixels(rendered))
        let want = try #require(Pixels(design))
        #expect(got.width == want.width && got.height == want.height)
        let off = got.differing(from: want, by: Self.channelTolerance)
        #expect(off <= Self.pixelTolerance, "\(frame.name): \(off * 100)% of pixels differ")
    }
}
