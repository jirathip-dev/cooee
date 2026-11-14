import AppKit
import ChatFormat
import SwiftUI

let folder = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
    .deletingLastPathComponent()

@MainActor
func write(_ view: some View, _ name: String) throws {
    let renderer = ImageRenderer(content: view)
    renderer.scale = 2
    guard let image = renderer.cgImage,
        let png = NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:])
    else { throw CocoaError(.fileWriteUnknown) }
    try png.write(to: folder.appendingPathComponent("\(name).png"))
    print(name, image.width, image.height)
}

try MainActor.assumeIsolated {
    for style in Style.allCases {
        for (mode, palette) in [("dark", Palette.farmDark), ("light", Palette.farmLight)] {
            try write(PhoneFrame(Look(palette: palette, style: style, phone: true)), "\(style.rawValue)-iphone-\(mode)")
        }
        let mac = style == .scene ? Palette.farmLight : Palette.farmDark
        try write(MacFrame(Look(palette: mac, style: style, phone: false)), "\(style.rawValue)-mac")
    }
}
