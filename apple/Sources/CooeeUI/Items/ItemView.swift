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
        case .link(_, let link):
            LinkLine(link: link)
        }
    }
}
