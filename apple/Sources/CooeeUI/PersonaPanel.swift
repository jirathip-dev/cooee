import CooeeCore
import SwiftUI

public struct PersonaPanel: View {
    @Environment(\.look) private var look
    private let persona: Persona
    private let content: PanelContent
    private let column: Bool
    private let end: Bool

    public var body: some View {
        let palette = look.palette
        VStack(spacing: 0) {
            if !column {
                Capsule().fill(palette.mute.opacity(0.5)).frame(width: 36, height: 5).padding(.top, 6)
            }
            header
            Scroller(end: end) {
                PanelSections(content: content).padding(.horizontal, 16).padding(.top, 4).padding(.bottom, 16)
            }
            footer.padding(.horizontal, 16).padding(.top, 10).padding(.bottom, column ? 16 : 30)
                .overlay(alignment: .top) { Rectangle().fill(palette.line).frame(height: 0.5) }
        }
        .background(background)
        .foregroundStyle(palette.foreground)
    }

    public init(personaID: String = "cond", column: Bool = false, end: Bool = false) {
        persona = Fixtures.persona(personaID)
        content = PanelContent.fixture(personaID)
        self.column = column
        self.end = end
    }
}

extension PersonaPanel {
    private var header: some View {
        HStack(spacing: 10) {
            Avatar(persona: persona, size: column ? 36 : 44)
            VStack(alignment: .leading, spacing: 3) {
                Text(persona.name).ui(column ? 16 : 18, .semibold, look)
                AgentChip(persona: persona)
            }
            Spacer(minLength: 4)
            if column {
                RoundButton(symbol: "sidebar.right", ascii: "]", size: 34).accessibilityLabel("Hide panel")
            } else {
                RoundButton(symbol: "xmark", ascii: "x", size: 36).accessibilityLabel("Close")
            }
        }
        .padding(.horizontal, 16).padding(.vertical, 10)
    }

    private var footer: some View {
        HStack(spacing: 10) {
            action("pencil", "Edit")
            action("square.and.pencil", "New chat")
            action("brain.head.profile", content.memory > 0 ? "Memory · \(content.memory)" : "Memory")
        }
    }

    private func action(_ symbol: String, _ title: String) -> some View {
        let palette = look.palette
        return VStack(spacing: 6) {
            if look.mono {
                Text("[\(title.lowercased())]").ui(13, .semibold, look)
            } else {
                Image(systemName: symbol).font(.system(size: 17, weight: .semibold))
                Text(title).ui(12, .medium, look).lineLimit(1)
            }
        }
        .foregroundStyle(palette.link).frame(maxWidth: .infinity).frame(height: 64)
        .background(
            palette.card.opacity(look.style == .modern ? 0.75 : 1), in: .rect(cornerRadius: look.mono ? 0 : 16)
        )
        .overlay {
            if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) }
        }
    }

    @ViewBuilder private var background: some View {
        let palette = look.palette
        if column {
            palette.background.overlay(alignment: .leading) { Rectangle().fill(palette.line).frame(width: 1) }
        } else {
            UnevenRoundedRectangle(topLeadingRadius: 34, topTrailingRadius: 34).fill(palette.background)
                .overlay(
                    UnevenRoundedRectangle(topLeadingRadius: 34, topTrailingRadius: 34)
                        .fill(palette.card.opacity(0.35))
                )
                .ignoresSafeArea()
        }
    }
}
