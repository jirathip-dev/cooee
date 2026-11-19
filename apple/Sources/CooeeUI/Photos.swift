import SwiftUI

public struct MessageActs {
    public var photos = FrameImages()
    public var source: (Int) -> String? = { _ in nil }
    public var quote: ((Int) -> Void)?

    public init() {}
}

private struct MessageActsKey: EnvironmentKey {
    fileprivate static var defaultValue: MessageActs { MessageActs() }
}

extension EnvironmentValues {
    internal var message: MessageActs {
        get { self[MessageActsKey.self] }
        set { self[MessageActsKey.self] = newValue }
    }
}

internal struct FileChip: View {
    @Environment(\.look) private var look
    @Environment(\.message) private var message
    @State private var open = false
    internal let name: String

    private static let photo = ["png", "jpg", "jpeg", "gif", "webp", "heic", "avif"]

    internal var body: some View {
        let data = message.photos.images[name]
        if let image = FrameThumb.image(data) {
            Button {
                open = true
            } label: {
                Image(decorative: image, scale: 1).resizable().scaledToFit()
                    .frame(maxWidth: 220, maxHeight: 220).clipShape(.rect(cornerRadius: 12))
            }
            .buttonStyle(.plain).accessibilityLabel(name)
            .viewer(isPresented: $open) { PhotoViewer(image: image) }
        } else if data == nil, Self.photo.contains(URL(filePath: name).pathExtension.lowercased()) {
            Color.clear.frame(width: 1, height: 1).task { message.photos.load(name) }
        } else {
            Label(name, systemImage: "doc.text").ui(13, .regular, look).foregroundStyle(look.palette.mute)
                .padding(.horizontal, 10).padding(.vertical, 6)
                .background(look.palette.card, in: .rect(cornerRadius: 10))
        }
    }
}

internal struct PhotoViewer: View {
    @Environment(\.dismiss) private var dismiss
    @State private var drag = CGFloat.zero
    internal let image: CGImage

    internal var body: some View {
        ZStack(alignment: .topTrailing) {
            Color.black.ignoresSafeArea()
            Image(decorative: image, scale: 1).resizable().scaledToFit()
                .frame(maxWidth: .infinity, maxHeight: .infinity).offset(y: drag)
            Button("Done") { dismiss() }.foregroundStyle(.white).padding()
        }
        .gesture(
            DragGesture()
                .onChanged { drag = max($0.translation.height, 0) }
                .onEnded { if $0.translation.height > 120 { dismiss() } else { drag = 0 } }
        )
    }
}

extension View {
    @ViewBuilder
    fileprivate func viewer(isPresented: Binding<Bool>, content: @escaping () -> some View) -> some View {
        #if os(iOS)
            fullScreenCover(isPresented: isPresented, content: content)
        #else
            sheet(isPresented: isPresented, content: content)
        #endif
    }
}
