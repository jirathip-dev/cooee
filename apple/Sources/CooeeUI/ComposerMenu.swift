import CooeeCore
import SwiftUI
import UniformTypeIdentifiers

internal struct ComposerMenu: View {
    @Environment(\.look) private var look
    private let picking = State(initialValue: false)
    internal let attach: ([Attachment]) -> Void
    internal let close: () -> Void

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 0) {
            Button {
                picking.wrappedValue = true
            } label: {
                HStack(spacing: 12) {
                    Mark(text: "[+]", size: 15) { Image(systemName: "paperclip").font(.system(size: 17)) }
                        .foregroundStyle(palette.link).frame(width: 28)
                    Text(look.label(.button, "Attach")).ui(16, .medium, look)
                    Spacer()
                }
                .padding(.horizontal, 16).frame(height: 48).contentShape(.rect)
            }
            .buttonStyle(.plain)
            Spacer(minLength: 0)
        }
        .padding(.top, 12).frame(minWidth: 280, minHeight: 72).foregroundStyle(palette.foreground)
        .presentationDetents([.height(120)])
        .fileImporter(isPresented: picking.projectedValue, allowedContentTypes: [.item], allowsMultipleSelection: true)
        { result in
            attach(((try? result.get()) ?? []).compactMap(Self.attachment))
            close()
        }
    }

    private static func attachment(_ url: URL) -> Attachment? {
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        return (try? Data(contentsOf: url)).map { Attachment(name: url.lastPathComponent, bytes: $0) }
    }
}
