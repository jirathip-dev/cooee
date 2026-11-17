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
            YouBubble(text: text, files: files).modifier(Copyable(text: text))
        case .outgoing(let message):
            VStack(alignment: .trailing, spacing: 4) {
                YouBubble(text: message.text, files: message.files.map(\.name))
                    .modifier(Copyable(text: message.text))
                Delivery(failure: message.failure, retry: retry)
            }
        case .agent(_, let blocks):
            VStack(alignment: .leading, spacing: 10) {
                ForEach(Array(blocks.enumerated()), id: \.offset) { _, block in BlockView(block: block) }
            }
            .modifier(Copyable(text: item.plainText ?? ""))
        case .tools(_, let rows):
            ToolFold(rows: rows)
        case .trace(_, let steps, let seconds):
            TraceRow(steps: steps, seconds: seconds)
        case .note(_, let text):
            HStack(spacing: 6) {
                Text(text).foregroundStyle(look.palette.mute)
                if text.hasPrefix("Updated ") {
                    Button("View", action: view).buttonStyle(.plain).foregroundStyle(look.palette.link)
                }
            }
            .ui(13, .regular, look).frame(maxWidth: .infinity)
        case .card(_, let poster, let block):
            BlockNode(block: block).environment(\.card, CardContext(poster: poster.map(LayoutSource.plugin) ?? .core))
        case .said(_, let speaker, let time, let blocks):
            Said(speaker: speaker, time: time, blocks: blocks)
        case .divider(_, let text):
            RoundDivider(text: text)
        case .asked(_, let ask):
            AskCard(ask: ask)
        }
    }
}

internal struct YouBubble: View {
    @Environment(\.look) private var look
    @Environment(\.members) private var members
    @Environment(\.noteTo) private var noteTo
    internal let text: String
    internal let files: [String]

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .trailing, spacing: 6) {
            if let noteTo {
                Text("Your note · to \(noteTo)").ui(13, .regular, look).foregroundStyle(palette.mute)
            }
            ForEach(files, id: \.self) { file in
                Label(file, systemImage: "doc.text").ui(13, .regular, look).foregroundStyle(palette.mute)
                    .padding(.horizontal, 10).padding(.vertical, 6)
                    .background(palette.card, in: .rect(cornerRadius: 10))
            }
            if members.isEmpty {
                Text(look.mono ? "> \(text)" : text).ui(16, .regular, look)
                    .padding(.horizontal, 14).padding(.vertical, 10)
                    .background(look.mono ? Color.clear : palette.link.opacity(0.22), in: .rect(cornerRadius: 20))
                    .foregroundStyle(look.mono ? palette.lead : palette.foreground)
            } else {
                roomBubble
            }
        }
        .frame(maxWidth: .infinity, alignment: .trailing).padding(.leading, look.mono ? 0 : 48)
    }

    private var roomBubble: some View {
        let palette = look.palette
        return Text(BlockView.mentioned(AttributedString(text), members, palette.link)).ui(16, .regular, look)
            .foregroundStyle(palette.foreground).padding(.horizontal, 14).padding(.vertical, 10)
            .frame(maxWidth: look.mono ? .infinity : nil, alignment: .leading)
            .background(
                look.mono ? palette.card : palette.amber.opacity(0.2), in: .rect(cornerRadius: look.mono ? 6 : 20)
            )
            .overlay(alignment: .leading) {
                if look.mono { Rectangle().fill(palette.lead).frame(width: 3) }
            }
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
    @Environment(\.members) private var members
    internal let block: Block

    internal var body: some View {
        switch block {
        case .text(let text):
            Text(Self.mentioned(Markdown.inline(text), members, look.palette.link)).ui(16, .regular, look)
                .fixedSize(horizontal: false, vertical: true)
        case .bullets(let bullets):
            VStack(alignment: .leading, spacing: 4) {
                ForEach(bullets, id: \.self) { bullet in
                    Text(Markdown.inline("•  " + bullet)).ui(16, .regular, look)
                }
            }
        case .code(let lang, let code):
            CodeBlock(lang: lang, code: code)
        }
    }

    internal static func mentioned(
        _ text: AttributedString, _ members: [Persona], _ tint: Color, token: Color? = nil
    ) -> AttributedString {
        var text = text
        let plain = String(text.characters)
        let cast = members.map(\.api)
        let typed = token != nil && MentionDraft(plain, members: cast).match != nil
        for mention in Mentions.all(plain)
        where Mentions.member(mention.key, cast) != nil || typed && mention.range.upperBound == plain.endIndex {
            let start = text.index(
                text.startIndex,
                offsetByCharacters: plain.distance(from: plain.startIndex, to: mention.range.lowerBound))
            let range = start..<text.index(start, offsetByCharacters: plain[mention.range].count)
            text[range].foregroundColor = tint
            if let token {
                text[range].backgroundColor = token
            } else {
                text[range].inlinePresentationIntent = .stronglyEmphasized
            }
        }
        return text
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
