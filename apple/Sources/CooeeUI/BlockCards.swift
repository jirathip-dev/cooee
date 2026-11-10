import CooeeCore
import SwiftUI

internal struct ActButton<Label: View>: View {
    @Environment(\.act) private var act
    @Environment(\.openURL) private var openURL
    @State private var pending: JSONValue?
    internal var action: String?
    internal let source: JSONValue
    internal var values: [String: JSONValue] = [:]
    @ViewBuilder internal let label: Label

    internal var body: some View {
        Button {
            if Confirm(for: source) == nil { fire() } else { pending = source }
        } label: {
            label
        }
        .buttonStyle(.plain)
        .modifier(Confirming(pending: $pending) { _ in fire() })
    }

    private func fire() {
        if let url = source["tap"]?.string("url").flatMap(URL.init(string:)) {
            openURL(url)
        } else if let action = BlockAction(action: action, source: source, values: values) {
            act(action)
        }
    }
}

internal struct Confirming: ViewModifier {
    @Binding internal var pending: JSONValue?
    internal let run: (JSONValue) -> Void

    internal func body(content: Content) -> some View {
        let confirm = pending.flatMap(Confirm.init(for:))
        content.confirmationDialog(
            confirm?.title ?? "",
            isPresented: Binding(get: { pending != nil }, set: { if !$0 { pending = nil } }),
            titleVisibility: .visible,
            presenting: pending
        ) { source in
            Button(confirm?.accept ?? "OK", role: confirm?.destructive == true ? .destructive : nil) { run(source) }
            Button("Cancel", role: .cancel) {}
        } message: { _ in
            if let message = confirm?.message { Text(message) }
        }
    }
}

internal struct CardBlock: View {
    @Environment(\.look) private var look
    @Environment(\.card) private var context
    internal let block: JSONValue

    private struct Kind {
        fileprivate let label: String
        fileprivate let symbol: String
        fileprivate let tint: Color
    }

    private var kind: Kind? {
        let palette = look.palette
        let poster = context.poster.map { " · \($0)" } ?? ""
        switch block.string("kind") {
        case "ask": return Kind(label: "ask\(poster)", symbol: "hand.raised.fill", tint: palette.red)
        case "config": return Kind(label: "config\(poster)", symbol: "slider.horizontal.3", tint: palette.cyan)
        case "secret": return Kind(label: "secret\(poster)", symbol: "key.fill", tint: palette.amber)
        case "run": return Kind(label: "run card\(poster)", symbol: "terminal.fill", tint: palette.lead)
        default: return nil
        }
    }

    internal var body: some View {
        let tint = kind?.tint ?? look.palette.tone(block.string("tone")) ?? look.palette.lead
        VStack(alignment: .leading, spacing: 10) {
            header(tint)
            if let title = block.string("title") {
                Text(title).ui(16, .semibold, look).foregroundStyle(look.palette.foreground)
            }
            if let subtitle = block.string("subtitle") {
                Text(subtitle).ui(13, .regular, look).foregroundStyle(look.palette.mute).lineLimit(2)
            }
            if !block.list("buttons").isEmpty {
                HStack(spacing: 8) {
                    ForEach(Array(block.list("buttons").enumerated()), id: \.offset) { _, button in
                        BlockNode(block: button)
                    }
                }
            }
            BlockStack(blocks: block.list("blocks")).environment(
                \.card, CardContext(poster: context.poster, tint: tint))
        }
        .padding(14)
        .surface(look, radius: 20)
        .overlay(alignment: .leading) {
            if look.style == .modern {
                RoundedRectangle(cornerRadius: 2).fill(tint).frame(width: 3).padding(.vertical, 16)
            }
        }
    }

    @ViewBuilder
    private func header(_ tint: Color) -> some View {
        let meta = block.string("meta")
        let badge = block["badge"]
        if kind != nil || meta != nil || badge != nil {
            HStack(spacing: 6) {
                if let kind {
                    if !look.mono { Image(systemName: kind.symbol).font(.system(size: 12, weight: .bold)) }
                    Text(look.mono ? "## \(kind.label)" : kind.label.uppercased()).ui(11, .bold, look)
                }
                Spacer(minLength: 4)
                if let badge { Pill(badge: badge) }
                if let meta { Text(meta).ui(12, .regular, look).foregroundStyle(look.palette.mute) }
            }
            .foregroundStyle(tint)
        }
    }
}

internal struct Pill: View {
    @Environment(\.look) private var look
    internal let badge: JSONValue

    internal var body: some View {
        let tint = look.palette.tone(badge.string("tone")) ?? look.palette.mute
        Text(badge.string("text") ?? "").ui(11, .bold, look).foregroundStyle(tint)
            .padding(.horizontal, 8).padding(.vertical, 3)
            .background(tint.opacity(0.15), in: look.mono ? AnyShape(Rectangle()) : AnyShape(Capsule()))
    }
}

internal struct AnswerBlock: View {
    internal let block: JSONValue

    internal var body: some View {
        switch block.type {
        case "choice": ChoiceBlock(block: block)
        case "secret": SecretBlock(block: block)
        default: RunBlock(block: block)
        }
    }
}

internal struct ChoiceBlock: View {
    @Environment(\.look) private var look
    @State private var other = ""
    internal let block: JSONValue

    private var options: [JSONValue] {
        let all = block.list("options")
        let recommended = block.string("recommended")
        return all.filter { $0.string("value") == recommended } + all.filter { $0.string("value") != recommended }
    }

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 8) {
            if let question = block.string("question") { Text(question).ui(15, .semibold, look) }
            ForEach(Array(options.enumerated()), id: \.offset) { index, option in
                ActButton(action: block.string("action"), source: option, values: values(option)) {
                    CardButton(
                        title: title(option),
                        primary: option.string("value") == block.string("recommended"),
                        tint: index == options.count - 1 && options.count > 1 ? palette.red : nil
                    )
                }
            }
            if let box = block["other"] {
                TextField(box.string("placeholder") ?? "", text: $other).ui(14, .regular, look)
                    .padding(.horizontal, 12).frame(height: 38)
                    .background(palette.background.opacity(0.5), in: .rect(cornerRadius: 10))
            }
            if let fallback = block["default"] {
                RelativeText(template: fallback.string("label") ?? "", until: fallback["until"]?.date)
                    .ui(12, .regular, look).foregroundStyle(palette.amber)
            }
        }
    }

    private func title(_ option: JSONValue) -> String {
        let label = option.string("label") ?? ""
        let recommended = option.string("value") == block.string("recommended") && !look.mono
        return recommended ? "\(label) · recommended" : label
    }

    private func values(_ option: JSONValue) -> [String: JSONValue] {
        let tied = block["other"]?.string("for")
        let text = tied == nil || tied == option.string("value") ? other : ""
        return ["choice": option["value"] ?? .null, "other": .string(text)]
    }
}
