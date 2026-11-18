import ImageIO
import SwiftUI

public struct FrameImages {
    public var images: [String: Data]
    public var load: @MainActor (String) -> Void

    public init(images: [String: Data] = [:], load: @escaping @MainActor (String) -> Void = { _ in }) {
        self.images = images
        self.load = load
    }
}

private struct FramesKey: EnvironmentKey {
    fileprivate static var defaultValue: FrameImages { FrameImages() }
}

extension EnvironmentValues {
    internal var frames: FrameImages {
        get { self[FramesKey.self] }
        set { self[FramesKey.self] = newValue }
    }
}

internal struct FrameThumb: View {
    @Environment(\.frames) private var frames
    internal let name: String

    internal var body: some View {
        if let image = Self.image(frames.images[name]) {
            Image(decorative: image, scale: 1).resizable().scaledToFit()
                .frame(maxWidth: 280, maxHeight: 180, alignment: .leading).clipShape(.rect(cornerRadius: 8))
                .accessibilityLabel("Browser frame")
        } else {
            Color.clear.frame(width: 1, height: 1).task { frames.load(name) }
        }
    }

    internal static func image(_ data: Data?) -> CGImage? {
        guard let data, let source = CGImageSourceCreateWithData(data as CFData, nil) else { return nil }
        return CGImageSourceCreateImageAtIndex(source, 0, nil)
    }
}
