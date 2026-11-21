import CooeeCore
import CooeeUI
import SwiftUI
import Testing

internal struct Frame: Sendable, CustomTestStringConvertible {
    internal static let phoneWidth: CGFloat = 402
    internal static let phoneHeight: CGFloat = 874

    internal let name: String
    internal let width: CGFloat
    internal let height: CGFloat
    internal let palette: Palette
    internal let phone: Bool
    internal var folder = "v1"
    internal let view: @MainActor @Sendable () -> AnyView

    internal var testDescription: String { name }

    @MainActor
    internal func render(_ style: Style = .bundled, margin: CGFloat = 0, around: Color = .clear) -> CGImage? {
        let look = Look(palette: palette, style: style, still: true)
        let content = Group {
            if phone { PhoneChrome { view() } } else { MacChrome { view() } }
        }
        .frame(width: width, height: height).background(palette.background)
        .environment(\.look, look).environment(\.colorScheme, palette.dark ? .dark : .light)
        .padding(margin).background(around)
        let renderer = ImageRenderer(content: content)
        renderer.scale = 2
        return renderer.cgImage
    }

    internal static func phone(
        _ name: String, _ palette: Palette, _ view: @escaping @MainActor @Sendable () -> some View
    ) -> Frame {
        Frame(
            name: name, width: phoneWidth, height: phoneHeight, palette: palette, phone: true,
            view: { AnyView(view()) })
    }

    internal static func mac(
        _ name: String, _ width: CGFloat, _ height: CGFloat, _ palette: Palette,
        _ view: @escaping @MainActor @Sendable () -> some View
    ) -> Frame {
        Frame(name: name, width: width, height: height, palette: palette, phone: false, view: { AnyView(view()) })
    }

    internal func filed(_ folder: String) -> Frame {
        var frame = self
        frame.folder = folder
        return frame
    }
}
