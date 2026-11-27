import AppKit
import CooeeCore
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

extension Frame {
    private static let channelTolerance = 16
    private static let pixelTolerance = 0.025
    private static let designs = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .appendingPathComponent("design")

    @MainActor
    internal func matchesItsDesign(_ style: Style = .bundled, in folder: String? = nil, only area: CGRect? = nil)
        throws
    {
        try Self.matches(try #require(render(style)), name, in: folder ?? self.folder, only: area)
    }

    internal static func matches(_ rendered: CGImage, _ name: String, in folder: String, only area: CGRect? = nil)
        throws
    {
        let file = designs.appendingPathComponent(folder).appendingPathComponent("\(name).png")
        let recording = ProcessInfo.processInfo.environment["COOEE_RECORD"] == "1" && area == nil
        if recording { try write(rendered, to: file) }
        try keep(rendered, name)
        let design = try #require(NSImage(contentsOf: file)?.cgImage(forProposedRect: nil, context: nil, hints: nil))
        let got = try #require(cropped(rendered, to: area).flatMap(Pixels.init))
        let want = try #require(cropped(design, to: area).flatMap(Pixels.init))
        #expect(got.width == want.width && got.height == want.height)
        let off = got.differing(from: want, by: channelTolerance)
        #expect(off <= pixelTolerance, "\(name): \(off * 100)% of pixels differ")
    }

    private static func cropped(_ image: CGImage, to area: CGRect?) -> CGImage? {
        guard let area else { return image }
        return image.cropping(
            to: CGRect(x: area.minX * 2, y: area.minY * 2, width: area.width * 2, height: area.height * 2))
    }

    @MainActor
    internal func fitsInEveryPack() throws {
        let margin = 24
        for style in Style.packs {
            let rendered = try #require(render(style, margin: CGFloat(margin)))
            try Self.keep(rendered, "\(name)@\(style.label.lowercased())")
            let pixels = try #require(Pixels(rendered))
            #expect(pixels.spilled(beyond: margin * 2) == 0, "\(name) draws outside its frame in \(style.label)")
        }
    }

    private static func write(_ image: CGImage, to file: URL) throws {
        let png = try #require(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
        try FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        try png.write(to: file)
    }

    private static func keep(_ image: CGImage, _ name: String) throws {
        guard let folder = ProcessInfo.processInfo.environment["COOEE_RENDERED"] else { return }
        try write(image, to: URL(fileURLWithPath: folder).appendingPathComponent("\(name).png"))
    }
}

@MainActor
internal struct SnapshotTests {
    @Test internal func aViewWiderThanItsFrameIsCaught() throws {
        let wide = Frame.mac("wide", 120, 60, .farmDark) { Text(String(repeating: "overflow ", count: 12)).fixedSize() }
        let rendered = try #require(wide.render(.terminal, margin: 24))
        let pixels = try #require(Pixels(rendered))
        #expect(pixels.spilled(beyond: 48) > 0)
    }
}
