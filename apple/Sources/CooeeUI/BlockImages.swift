import CooeeCore
import CoreImage
import SwiftUI

internal struct ImagesBlock: View {
    @State private var open: Int?
    internal let block: JSONValue

    internal var body: some View {
        let items = block.list("items")
        let side: CGFloat = block.string("size") == "small" ? 72 : 160
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(Array(items.enumerated()), id: \.offset) { index, item in
                    Button {
                        open = index
                    } label: {
                        Picture(item: item).frame(width: side, height: side).clipShape(.rect(cornerRadius: 12))
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .sheet(item: Binding(get: { open.map(Opened.init) }, set: { open = $0?.index })) { opened in
            ZoomedPicture(item: items[opened.index])
        }
    }
}

private struct Opened: Identifiable {
    fileprivate let index: Int
    fileprivate var id: Int { index }
}

private struct ZoomedPicture: View {
    @State private var scale: CGFloat = 1
    fileprivate let item: JSONValue

    fileprivate var body: some View {
        Picture(item: item, fit: true).scaleEffect(scale)
            .gesture(MagnifyGesture().onChanged { scale = max(1, $0.magnification) })
    }
}

private struct Picture: View {
    @Environment(\.look) private var look
    fileprivate let item: JSONValue
    fileprivate var fit = false

    fileprivate var body: some View {
        let src = item.string("src") ?? ""
        if let image = Self.decoded(src) {
            Image(decorative: image, scale: 1).resizable().aspectRatio(contentMode: fit ? .fit : .fill)
        } else if let url = URL(string: src), url.scheme == "https" || url.scheme == "http" {
            AsyncImage(url: url) {
                $0.resizable().aspectRatio(contentMode: fit ? .fit : .fill)
            } placeholder: {
                alt
            }
        } else {
            alt
        }
    }

    private var alt: some View {
        Text(item.string("alt") ?? "").ui(12, .regular, look).foregroundStyle(look.palette.mute)
            .frame(maxWidth: .infinity, maxHeight: .infinity).background(look.palette.card)
    }

    private static func decoded(_ src: String) -> CGImage? {
        guard src.hasPrefix("data:image/"), let comma = src.firstIndex(of: ","),
            let data = Data(base64Encoded: String(src[src.index(after: comma)...])),
            let image = CIImage(data: data)
        else { return nil }
        return CIContext().createCGImage(image, from: image.extent)
    }
}
