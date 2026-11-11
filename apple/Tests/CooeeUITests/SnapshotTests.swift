import AppKit
import CooeeUI
import Foundation
import SwiftUI
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

    private static let opaque: UInt8 = 200

    internal func spilled(beyond inset: Int) -> Int {
        let rows = 0..<height
        let columns = 0..<width
        return rows.reduce(0) { total, row in
            total
                + columns.count { column in
                    let inside = (inset..<width - inset).contains(column) && (inset..<height - inset).contains(row)
                    let start = (row * width + column) * 4
                    return !inside && bytes[start + 3] > Self.opaque
                }
        }
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
        if ProcessInfo.processInfo.environment["COOEE_RECORD"] == "1" { try Self.write(rendered, to: file) }
        try Self.keep(rendered, frame.name)
        let design = try #require(NSImage(contentsOf: file)?.cgImage(forProposedRect: nil, context: nil, hints: nil))
        let got = try #require(Pixels(rendered))
        let want = try #require(Pixels(design))
        #expect(got.width == want.width && got.height == want.height)
        let off = got.differing(from: want, by: Self.channelTolerance)
        #expect(off <= Self.pixelTolerance, "\(frame.name): \(off * 100)% of pixels differ")
    }

    @Test(arguments: Frame.everyStyle)
    internal func frameFitsInEveryStyle(_ frame: Frame) throws {
        let margin = 24
        let rendered = try #require(frame.render(margin: CGFloat(margin)))
        try Self.keep(rendered, frame.name)
        let pixels = try #require(Pixels(rendered))
        #expect(pixels.spilled(beyond: margin * 2) == 0, "\(frame.name) draws outside its frame")
    }

    @Test internal func aViewWiderThanItsFrameIsCaught() throws {
        let wide = Frame(
            name: "wide", width: 120, height: 60, palette: .farmDark, style: .terminal, phone: false,
            view: { AnyView(Text(String(repeating: "overflow ", count: 12)).fixedSize()) }
        )
        let rendered = try #require(wide.render(margin: 24))
        let pixels = try #require(Pixels(rendered))
        #expect(pixels.spilled(beyond: 48) > 0)
    }

    private static func write(_ image: CGImage, to file: URL) throws {
        let png = try #require(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
        try png.write(to: file)
    }

    private static func keep(_ image: CGImage, _ name: String) throws {
        guard let folder = ProcessInfo.processInfo.environment["COOEE_SNAPSHOT_OUT"] else { return }
        let url = URL(fileURLWithPath: folder)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        try write(image, to: url.appendingPathComponent("\(name).png"))
    }
}
