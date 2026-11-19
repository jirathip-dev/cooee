import CooeeCore
import SwiftUI

internal struct ItemView: View, Equatable {
    @Environment(\.look) private var look
    internal let item: Item
    internal var retry: () -> Void = {}
    internal var view: () -> Void = {}
    internal var restart: (() -> Void)?

    nonisolated internal static func == (lhs: Self, rhs: Self) -> Bool {
        lhs.item == rhs.item && (lhs.restart == nil) == (rhs.restart == nil)
    }

    internal var body: some View {
        switch item {
        case .you(let id, let text, let files):
            YouBubble(text: text, files: files).modifier(Copyable(text: text, id: id))
        case .outgoing(let message):
            VStack(alignment: .trailing, spacing: 4) {
                YouBubble(text: message.text, files: message.files.map(\.name))
                    .modifier(Copyable(text: message.text, id: message.id))
                Delivery(failure: message.failure, retry: retry)
            }
        case .agent(_, let blocks):
            VStack(alignment: .leading, spacing: 10) {
                ForEach(Array(blocks.enumerated()), id: \.offset) { _, block in BlockView(block: block).equatable() }
            }
            .modifier(Copyable(text: item.plainText ?? "", id: item.id))
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
                if text.hasPrefix("The agent is gone"), let restart {
                    Button("Restart", action: restart).buttonStyle(.plain).foregroundStyle(look.palette.link)
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
        case .screen(_, let screen):
            ScreenItemView(screen: screen)
        case .call(let call):
            CallBlock(call: call)
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
            ForEach(files, id: \.self) { FileChip(name: $0) }
            if members.isEmpty {
                Text(look.label(.said, text)).ui(16, .regular, look)
                    .padding(.horizontal, 14).padding(.vertical, 10)
                    .background(look.outline ? Color.clear : palette.link.opacity(0.22), in: .rect(cornerRadius: 20))
                    .foregroundStyle(look.outline ? palette.lead : palette.foreground)
            } else {
                roomBubble
            }
        }
        .frame(maxWidth: .infinity, alignment: .trailing).padding(.leading, look.outline ? 0 : 48)
    }

    private var roomBubble: some View {
        let palette = look.palette
        return Text(BlockView.mentioned(AttributedString(text), members, palette.link)).ui(16, .regular, look)
            .foregroundStyle(palette.foreground).padding(.horizontal, 14).padding(.vertical, 10)
            .frame(maxWidth: look.outline ? .infinity : nil, alignment: .leading)
            .accent(
                look, palette.lead, fill: palette.amber.opacity(0.2), radius: look.radius(20, least: 6),
                paper: palette.card)
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

internal struct BlockView: View, Equatable {
    @Environment(\.look) private var look
    @Environment(\.members) private var members
    internal let block: Block

    nonisolated internal static func == (lhs: Self, rhs: Self) -> Bool { lhs.block == rhs.block }

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
        .background(palette.card, in: .rect(cornerRadius: look.radius(12)))
        .overlay(RoundedRectangle(cornerRadius: look.radius(12)).strokeBorder(palette.edge, lineWidth: 1))
    }
}

internal struct ToolFold: View {
    @Environment(\.look) private var look
    @Environment(\.members) private var members
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
                if let frame = last.frame { FrameThumb(name: frame) }
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
            mark(last)
            Text(count > 0 && !members.isEmpty ? "\(last.title) · \(count) tools · only you see these" : last.title)
                .ui(13, .regular, look).foregroundStyle(palette.mute).lineLimit(1).minimumScaleFactor(0.85)
                .truncationMode(members.isEmpty ? .middle : .tail)
            Spacer(minLength: 4)
            if count > 0 {
                if members.isEmpty { Text("\(count) tools").ui(12, .regular, look).foregroundStyle(palette.mute) }
                Image(systemName: open.wrappedValue ? "chevron.down" : "chevron.right").font(
                    .system(size: 10, weight: .bold)
                )
                .foregroundStyle(palette.mute)
            }
        }
        .contentShape(.rect)
        .padding(.horizontal, 12).frame(height: 34)
        .chip(look, fill: palette.card.opacity(0.7), edge: palette.line, radius: 10)
    }

    @ViewBuilder private func mark(_ last: ToolRow) -> some View {
        let tint = last.running ? look.palette.cyan : look.palette.green
        Mark(text: last.running ? "o>_" : members.isEmpty ? "[ok]" : "✓") {
            Image(systemName: last.running ? "gearshape.2.fill" : "checkmark.circle.fill").font(.system(size: 13))
        }
        .foregroundStyle(tint).fixedSize()
    }
}
