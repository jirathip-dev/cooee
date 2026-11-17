import CooeeCore
import ImageIO
import UniformTypeIdentifiers

extension DrawEdit {
    internal static func imported(_ url: URL, _ figure: PixelFigure, _ tokens: [String: String]) -> DrawEdit? {
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        guard let data = try? Data(contentsOf: url) else { return nil }
        if UTType(filenameExtension: url.pathExtension)?.conforms(to: .svg) == true {
            return String(bytes: data, encoding: .utf8).flatMap(PixelFigure.vector).map(DrawEdit.vector)
        }
        return pixels(data, figure.width, figure.height).map { .cells(PixelFigure.nearest($0, tokens)) }
    }

    private static func pixels(_ data: Data, _ width: Int, _ height: Int) -> [UInt8]? {
        let upright: [CFString: CFTypeRef] = [
            kCGImageSourceCreateThumbnailFromImageAlways: kCFBooleanTrue,
            kCGImageSourceCreateThumbnailWithTransform: kCFBooleanTrue,
            kCGImageSourceThumbnailMaxPixelSize: 1024 as CFNumber,
        ]
        guard let source = CGImageSourceCreateWithData(data as CFData, nil),
            let image = CGImageSourceCreateThumbnailAtIndex(source, 0, upright as CFDictionary)
        else { return nil }
        let scale = min(Double(width) / Double(image.width), Double(height) / Double(image.height))
        let (across, down) = (Double(image.width) * scale, Double(image.height) * scale)
        var rgba = [UInt8](repeating: 0, count: width * height * 4)
        let drawn = rgba.withUnsafeMutableBytes { buffer in
            let context = CGContext(
                data: buffer.baseAddress, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
                space: CGColorSpace(name: CGColorSpace.sRGB) ?? CGColorSpaceCreateDeviceRGB(),
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
            )
            context?.draw(image, in: CGRect(x: (Double(width) - across) / 2, y: 0, width: across, height: down))
            return context != nil
        }
        return drawn ? rgba : nil
    }
}
