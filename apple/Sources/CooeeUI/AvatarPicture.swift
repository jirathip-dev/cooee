import ImageIO
import SwiftUI
import UniformTypeIdentifiers

internal enum AvatarPicture {
    internal static func dataURL(_ data: Data, side: Int = 256) -> String? {
        let thumbnail: [CFString: Bool] = [
            kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceCreateThumbnailWithTransform: true,
        ]
        var options = thumbnail as [CFString: CFTypeRef]
        options[kCGImageSourceThumbnailMaxPixelSize] = side as CFNumber
        let out = NSMutableData()
        guard let source = CGImageSourceCreateWithData(data as CFData, nil),
            let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary),
            let target = CGImageDestinationCreateWithData(out, UTType.jpeg.identifier as CFString, 1, nil)
        else { return nil }
        CGImageDestinationAddImage(target, image, nil)
        guard CGImageDestinationFinalize(target) else { return nil }
        return "data:image/jpeg;base64," + (out as Data).base64EncodedString()
    }

    internal static func image(_ avatar: String?) -> CGImage? {
        guard let avatar, avatar.hasPrefix("data:image/"), let comma = avatar.firstIndex(of: ","),
            let data = Data(base64Encoded: String(avatar[avatar.index(after: comma)...])),
            let source = CGImageSourceCreateWithData(data as CFData, nil)
        else { return nil }
        return CGImageSourceCreateImageAtIndex(source, 0, nil)
    }
}
