import CooeeCore
@testable import CooeeUI
import Foundation
import ImageIO
import Testing
import UniformTypeIdentifiers

internal struct ThemeImportTests {
    private static let tokens = ["--fk-l": "#ff9e80", "--fk-c": "#4fd1c5"]

    private static func picture(_ type: UTType, _ rows: [[UInt32]], in folder: URL) throws -> URL {
        let (width, height) = (rows[0].count, rows.count)
        let context = try #require(
            CGContext(
                data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
                space: CGColorSpace(name: CGColorSpace.sRGB) ?? CGColorSpaceCreateDeviceRGB(),
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
            ))
        for (row, line) in rows.enumerated() {
            for (col, rgb) in line.enumerated() {
                let channels = [rgb >> 16, rgb >> 8 & 255, rgb & 255].map { CGFloat($0) / 255 }
                context.setFillColor(red: channels[0], green: channels[1], blue: channels[2], alpha: 1)
                context.fill(CGRect(x: col, y: height - 1 - row, width: 1, height: 1))
            }
        }
        let url = folder.appendingPathComponent("art").appendingPathExtension(for: type)
        let target = try #require(CGImageDestinationCreateWithURL(url as CFURL, type.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(target, try #require(context.makeImage()), nil)
        #expect(CGImageDestinationFinalize(target))
        return url
    }

    private static func cells(_ edit: DrawEdit?) -> [Int?] {
        if case .cells(let cells) = edit { cells } else { [] }
    }

    @Test internal func aPngOrPhotoSnapsOntoTheGridAndAnSvgStaysVector() throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let figure = PixelFigure(blank: "s-needs")
        let wide = try Self.picture(.png, [[0xff9e80, 0xff9e80], [0x4fd1c5, 0x4fd1c5]].map { $0 + $0 }, in: folder)
        let png = Self.cells(DrawEdit.imported(wide, figure, Self.tokens))
        #expect(png.count == 24 * 24 && png.prefix(24 * 12).allSatisfy { $0 == nil })
        #expect(png[12 * 24] == 0 && png[13 * 24 + 23] == 0 && png[23 * 24] == 1 && png[24 * 24 - 1] == 1)
        let photo = try Self.picture(.jpeg, [[0xfafafa, 0x0e1418], [0x50d0c0, 0xfa9a80]], in: folder)
        let jpeg = Self.cells(DrawEdit.imported(photo, figure, Self.tokens))
        #expect([jpeg[0], jpeg[23], jpeg[23 * 24], jpeg[24 * 24 - 1]] == [8, 9, 1, 0])
        let svg = folder.appendingPathComponent("art.svg")
        try Data(#"<svg viewBox="0 0 8 8"><rect width="8" height="8"/></svg>"#.utf8).write(to: svg)
        let vector = #"<svg viewBox="0 0 8 8" width="100%" height="100%"><rect width="8" height="8"/></svg>"#
        #expect(DrawEdit.imported(svg, figure, Self.tokens) == .vector(vector))
        try Data("not a picture".utf8).write(to: folder.appendingPathComponent("art.png"))
        #expect(DrawEdit.imported(wide, figure, Self.tokens) == nil)
    }
}
