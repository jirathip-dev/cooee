import CooeeCore
import SwiftUI

internal struct ItemView: View {
    @Environment(\.look) private var look
    internal let item: Item
    internal var retry: () -> Void = {}
    internal var view: () -> Void = {}

    internal var body: some View {
        switch item {
        case .you(_, let text, let files):
            YouBubble(text: text, files: files)
        case .outgoing(let message):
            VStack(alignment: .trailing, spacing: 4) {
                YouBubble(text: message.text, files: message.files.map(\.name))
                Delivery(failure: message.failure, retry: retry)
            }
        case .agent(_, let blocks):
            VStack(alignment: .leading, spacing: 10) {
                ForEach(Array(blocks.enumerated()), id: \.offset) { _, block in BlockView(block: block) }
            }
        case .tools(_, let rows):
            ToolFold(rows: rows)
        case .note(_, let text):
            HStack(spacing: 6) {
                Text(text).foregroundStyle(look.palette.mute)
                if text.hasPrefix("Updated ") {
                    Button("View", action: view).buttonStyle(.plain).foregroundStyle(look.palette.link)
                }
            }
            .ui(13, .regular, look).frame(maxWidth: .infinity)
        case .card(_, let poster, let block):
            BlockNode(block: block).environment(\.card, CardContext(poster: poster))
        }
    }
}

internal struct YouBubble: View {
    @Environment(\.look) private var look
    internal let text: String
    internal let files: [String]

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .trailing, spacing: 6) {
            ForEach(files, id: \.self) { file in
                Label(file, systemImage: "doc.text").ui(13, .regular, look).foregroundStyle(palette.mute)
                    .padding(.horizontal, 10).padding(.vertical, 6)
                    .background(palette.card, in: .rect(cornerRadius: 10))
            }
            Text(look.mono ? "> \(text)" : text).ui(16, .regular, look)
                .padding(.horizontal, 14).padding(.vertical, 10)
                .background(look.mono ? Color.clear : palette.link.opacity(0.22), in: .rect(cornerRadius: 20))
                .foregroundStyle(look.mono ? palette.lead : palette.foreground)
        }
        .frame(maxWidth: .infinity, alignment: .trailing).padding(.leading, look.mono ? 0 : 48)
    }
}

internal struct Delivery: View {
    @Environment(\.look) private var look
    internal let failure: String?
    internal let retry: () -> Void

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .trailing, spacing: 2) {
            if let failure {
                Button("Not delivered · Retry", action: retry).buttonStyle(.plain).ui(12, .semibold, look)
                    .foregroundStyle(palette.red)
                Text(failure).ui(12, .regular, look).foregroundStyle(palette.mute)
            } else {
                Text("Sending…").ui(12, .regular, look).foregroundStyle(palette.mute)
            }
        }
        .frame(maxWidth: .infinity, alignment: .trailing)
    }
}

internal struct BlockView: View {
    @Environment(\.look) private var look
    internal let block: Block

    internal var body: some View {
        switch block {
        case .text(let text):
            Text(Self.marked(text)).ui(16, .regular, look).fixedSize(horizontal: false, vertical: true)
        case .bullets(let bullets):
            VStack(alignment: .leading, spacing: 4) {
                ForEach(bullets, id: \.self) { bullet in
                    Text(Self.marked("•  " + bullet, plain: bullet)).ui(16, .regular, look)
                }
            }
        case .code(let lang, let code):
            CodeBlock(lang: lang, code: code)
        }
    }

    internal static func marked(_ markdown: String, plain: String? = nil) -> AttributedString {
        (try? AttributedString(markdown: markdown)) ?? AttributedString(plain ?? markdown)
    }
}

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
            Text(code).font(.system(size: 13, design: .monospaced)).foregroundStyle(palette.foreground)
                .padding(12).frame(maxWidth: .infinity, alignment: .leading)
                .fixedSize(horizontal: false, vertical: true)
        }
        .background(palette.card, in: .rect(cornerRadius: look.mono ? 0 : 12))
        .overlay(RoundedRectangle(cornerRadius: look.mono ? 0 : 12).strokeBorder(palette.edge, lineWidth: 1))
    }
}

internal struct ToolFold: View {
    @Environment(\.look) private var look
    private let open = State(initialValue: false)
    internal let rows: [ToolRow]

    internal var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            if let last = rows.last {
                Button {
                    open.wrappedValue.toggle()
                } label: {
                    row(last, count: rows.count)
                }
                .buttonStyle(.plain)
            }
            if open.wrappedValue {
                ForEach(Array(rows.dropLast().enumerated()), id: \.offset) { _, earlier in
                    row(earlier, count: 0).padding(.leading, 12)
                }
            }
        }
    }

    private func row(_ last: ToolRow, count: Int) -> some View {
        let palette = look.palette
        return HStack(spacing: 8) {
            if look.mono {
                Text(last.running ? "o>_" : "[ok]").ui(13, .bold, look)
                    .foregroundStyle(last.running ? palette.cyan : palette.green)
            } else {
                Image(systemName: last.running ? "gearshape.2.fill" : "checkmark.circle.fill")
                    .font(.system(size: 13)).foregroundStyle(last.running ? palette.cyan : palette.green)
            }
            Text(last.title).ui(13, .regular, look).foregroundStyle(palette.mute).lineLimit(1).truncationMode(.middle)
            Spacer(minLength: 4)
            if count > 0 {
                Text("\(count) tools").ui(12, .regular, look).foregroundStyle(palette.mute)
                Image(systemName: open.wrappedValue ? "chevron.down" : "chevron.right").font(
                    .system(size: 10, weight: .bold)
                )
                .foregroundStyle(palette.mute)
            }
        }
        .contentShape(.rect)
        .padding(.horizontal, 12).frame(height: 34)
        .background(palette.card.opacity(look.mono ? 0 : 0.7), in: .rect(cornerRadius: look.mono ? 0 : 10))
        .overlay {
            if look.mono { Rectangle().strokeBorder(palette.line, lineWidth: 1) }
        }
    }
}
