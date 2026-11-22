import SwiftUI

internal struct CodeBlock: View {
    @Environment(\.look) private var look
    internal let lang: String
    internal let code: String

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text(lang).font(.system(size: 12, design: .monospaced)).foregroundStyle(palette.mute)
                Spacer()
                Button {
                    copy(code)
                } label: {
                    Label("Copy", systemImage: "doc.on.doc").ui(12, .medium, look).foregroundStyle(palette.link)
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 12).frame(height: 32)
            .background(palette.edge.opacity(0.35))
            Text(lang == "diff" ? Self.diff(code, palette) : AttributedString(code))
                .font(.system(size: 13, design: .monospaced)).foregroundStyle(palette.foreground)
                .padding(12).frame(maxWidth: .infinity, alignment: .leading)
                .fixedSize(horizontal: false, vertical: true)
        }
        .background(palette.card, in: .rect(cornerRadius: look.radius(12)))
        .overlay(RoundedRectangle(cornerRadius: look.radius(12)).strokeBorder(palette.edge, lineWidth: 1))
    }
}
