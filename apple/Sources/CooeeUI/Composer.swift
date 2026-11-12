import CooeeCore
import SwiftUI
import UniformTypeIdentifiers

internal struct Composer: View {
    @Environment(\.look) private var look
    @Environment(\.members) private var members
    private let picking = State(initialValue: false)
    private let sending = State(initialValue: false)
    @FocusState private var typing: Bool
    internal let hint: String
    internal var queued: [String] = []
    internal var draft = ""
    internal var attachments: [String] = []
    internal var actions = ChatActions()

    private var chips: [String] { actions.send == nil ? attachments : actions.files }
    private var shown: String { queued.isEmpty ? hint : "Queue a message" }

    internal var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(Array(queued.enumerated()), id: \.offset) { index, text in queuedRow(index, text) }
            if !chips.isEmpty { chipRow }
            HStack(alignment: .bottom, spacing: 8) {
                Button {
                    picking.wrappedValue = true
                } label: {
                    RoundButton(symbol: "plus", ascii: "+", size: 42)
                }
                .buttonStyle(.plain).accessibilityLabel("Attach")
                field
                sendButton
            }
        }
        .fileImporter(isPresented: picking.projectedValue, allowedContentTypes: [.item], allowsMultipleSelection: true)
        { result in
            actions.attach(((try? result.get()) ?? []).compactMap(Self.attachment))
        }
    }

    private var field: some View {
        let palette = look.palette
        return HStack {
            if let text = actions.draft {
                TextField(shown, text: text, axis: .vertical).ui(16, .regular, look).lineLimit(1...5)
                    .textFieldStyle(.plain).onSubmit(send).focused($typing)
                    .onChange(of: typing) { _, now in if !now && sending.wrappedValue { post() } }
            } else {
                Text(draft.isEmpty ? shown : draft).ui(16, .regular, look)
                    .foregroundStyle(draft.isEmpty ? palette.mute : palette.foreground)
            }
            Spacer()
            if members.isEmpty { Image(systemName: "mic").foregroundStyle(palette.mute) }
        }
        .padding(.horizontal, 16).frame(minHeight: 42)
        .surface(look, radius: 21)
    }

    private var sendButton: some View {
        let palette = look.palette
        return Button(action: send) {
            Image(systemName: "arrow.up").font(.system(size: 17, weight: .bold))
                .foregroundStyle(palette.dark ? Color.black : .white)
                .frame(width: 42, height: 42)
                .background(palette.link, in: look.mono ? AnyShape(Rectangle()) : AnyShape(Circle()))
        }
        .buttonStyle(.plain).accessibilityLabel("Send")
    }

    private var chipRow: some View {
        let palette = look.palette
        return HStack(spacing: 6) {
            ForEach(Array(chips.enumerated()), id: \.offset) { index, name in
                HStack(spacing: 6) {
                    Image(systemName: name.hasSuffix(".png") ? "photo" : "doc")
                    Text(name)
                    Button {
                        actions.detach(index)
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                    }
                    .buttonStyle(.plain)
                }
                .ui(12, .regular, look).foregroundStyle(palette.mute)
                .padding(.horizontal, 10).frame(height: 28).background(palette.card, in: .capsule)
            }
        }
    }

    private func queuedRow(_ index: Int, _ text: String) -> some View {
        let palette = look.palette
        return HStack(spacing: 8) {
            Image(systemName: "clock").font(.system(size: 12)).foregroundStyle(palette.mute)
            Text(text).ui(14, .regular, look).lineLimit(1)
            Spacer()
            Button {
                actions.steer(index)
            } label: {
                Text("Send now").ui(12, .semibold, look).foregroundStyle(palette.link)
            }
            .buttonStyle(.plain)
            Button {
                actions.unqueue(index)
            } label: {
                Image(systemName: "xmark").font(.system(size: 11, weight: .bold)).foregroundStyle(palette.mute)
            }
            .buttonStyle(.plain).accessibilityLabel("Take back")
        }
        .padding(.horizontal, 12).frame(height: 34)
        .background(palette.card.opacity(0.8), in: .rect(cornerRadius: 12))
    }

    private func send() {
        if typing {
            sending.wrappedValue = true
            typing = false
        } else {
            post()
        }
    }

    private func post() {
        sending.wrappedValue = false
        actions.send?()
    }

    private static func attachment(_ url: URL) -> Attachment? {
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        return (try? Data(contentsOf: url)).map { Attachment(name: url.lastPathComponent, bytes: $0) }
    }
}
