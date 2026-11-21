import CooeeCore
import SwiftUI

internal struct MentionBar: View {
    @Environment(\.look) private var look
    internal let scene: RoomScene
    internal var draft: Binding<String>?

    internal var body: some View {
        let mention = scene.mention
        VStack(alignment: .leading, spacing: 18) {
            if mention.query != nil { popover(mention) }
            if let line = Mentions.line(mention.order.compactMap(name)) {
                Text(Self.inked(line, look.palette.foreground)).ui(13, .regular, look)
                    .foregroundStyle(look.palette.mute).fixedSize(horizontal: false, vertical: true)
                    .padding(.horizontal, 4).padding(.bottom, 8)
            }
        }
    }

    private func name(_ id: String) -> String? {
        scene.members.first { $0.id == id }?.name
    }

    private func popover(_ mention: MentionDraft) -> some View {
        let palette = look.palette
        let card = RoundedRectangle(cornerRadius: look.radius(22))
        let rows =
            scene.members.filter { $0.id == mention.match }
            + scene.members.filter { $0.id != mention.match && !mention.named.contains($0.id) }
            + scene.members.filter { mention.named.contains($0.id) }
        return VStack(alignment: .leading, spacing: 0) {
            Text("Members").ui(13, .regular, look).foregroundStyle(palette.mute)
                .padding(.horizontal, 14).frame(height: 52)
            ForEach(rows, id: \.id) { member in row(member, mention) }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(palette.card.opacity(0.6), in: card).clipShape(card)
        .overlay(card.strokeBorder(palette.line, lineWidth: 1))
    }

    private func row(_ member: Persona, _ mention: MentionDraft) -> some View {
        let palette = look.palette
        let named = mention.named.contains(member.id)
        let note =
            named
            ? "mentioned"
            : member.id == mention.match ? "replies \(RoomOrder.ordinal(mention.named.count + 1))" : member.api.machine
        return Button {
            draft?.wrappedValue = Mentions.pick(member.name, in: scene.draft)
        } label: {
            HStack(spacing: 10) {
                FigureView(persona: member.with(state: .done), size: 30)
                Text(member.name).ui(15, .regular, look).lineLimit(1)
                Spacer(minLength: 8)
                Text(note).ui(12, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
            }
            .padding(.horizontal, 14).frame(height: 48).contentShape(.rect).opacity(named ? 0.45 : 1)
        }
        .buttonStyle(.plain).accessibilityLabel("Mention \(member.name)")
        .background(Rectangle().fill(member.id == mention.match ? palette.foreground.opacity(0.07) : .clear))
    }

    private static func inked(_ text: AttributedString, _ ink: Color) -> AttributedString {
        var text = text
        for run in text.runs where run.inlinePresentationIntent == .stronglyEmphasized {
            text[run.range].foregroundColor = ink
        }
        return text
    }
}
