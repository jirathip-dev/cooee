import CooeeCore
import SwiftUI

internal struct ChoiceCard: View {
    @Environment(\.look) private var look
    internal let choice: Choice

    internal var body: some View {
        let palette = look.palette
        ChatCard(kind: "ask", symbol: "hand.raised.fill", tint: palette.red, title: choice.title) {
            Text(choice.detail).ui(13, .regular, look).foregroundStyle(palette.mute).lineLimit(2)
            VStack(spacing: 8) {
                ForEach(Array(choice.options.enumerated()), id: \.offset) { index, option in
                    CardButton(
                        title: index == choice.recommended && !look.mono ? "\(option) · recommended" : option,
                        primary: index == choice.recommended,
                        tint: index == choice.options.count - 1 ? palette.red : nil
                    )
                }
            }
        }
    }
}

internal struct ConfigCard: View {
    @Environment(\.look) private var look
    internal let plugin: String
    internal let title: String
    internal let fields: [Field]

    internal var body: some View {
        let palette = look.palette
        ChatCard(kind: "config · \(plugin)", symbol: "slider.horizontal.3", tint: palette.cyan, title: title) {
            ForEach(fields, id: \.key) { field in
                HStack {
                    Text(field.key).ui(14, .regular, look).foregroundStyle(palette.mute)
                    Spacer()
                    Text(field.value).ui(14, .regular, look)
                }
                .padding(.horizontal, 12).frame(height: 38)
                .background(palette.background.opacity(0.5), in: .rect(cornerRadius: 10))
            }
            CardButton(title: "Save", primary: true, tint: palette.cyan)
        }
    }
}

internal struct SecretCard: View {
    @Environment(\.look) private var look
    internal let plugin: String
    internal let name: String
    internal let why: String

    internal var body: some View {
        let palette = look.palette
        ChatCard(kind: "secret · \(plugin)", symbol: "key.fill", tint: palette.amber, title: name) {
            HStack {
                Text("••••••••••••").ui(15, .regular, look).foregroundStyle(palette.mute)
                Spacer()
                Image(systemName: "faceid").foregroundStyle(palette.mute)
            }
            .padding(.horizontal, 12).frame(height: 38)
            .background(palette.background.opacity(0.5), in: .rect(cornerRadius: 10))
            Label(why, systemImage: "lock.fill").ui(12, .regular, look).foregroundStyle(palette.mute)
            CardButton(title: "Send to plugin", primary: true, tint: palette.amber)
        }
    }
}

internal struct RunCardView: View {
    @Environment(\.look) private var look
    internal let card: RunCard

    internal var body: some View {
        let palette = look.palette
        ChatCard(kind: "run card", symbol: "terminal.fill", tint: palette.lead, title: card.why) {
            Text("$ \(card.command)").font(.system(size: 13, design: .monospaced)).foregroundStyle(palette.foreground)
                .padding(10).frame(maxWidth: .infinity, alignment: .leading)
                .background(palette.background.opacity(0.6), in: .rect(cornerRadius: 10))
            VStack(alignment: .leading, spacing: 3) {
                Text("sha \(card.sha)").font(.system(size: 12, design: .monospaced))
                Text("in \(card.folder)")
                Text("touches \(card.blast)")
            }
            .ui(12, .regular, look).foregroundStyle(palette.mute)
            HStack(spacing: 8) {
                CardButton(title: "Approve", primary: true, tint: palette.green)
                CardButton(title: "Deny", tint: palette.red)
            }
            Text(card.lapses).ui(12, .regular, look).foregroundStyle(palette.amber)
        }
    }
}
