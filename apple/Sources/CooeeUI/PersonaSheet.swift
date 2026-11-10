import CooeeCore
import SwiftUI

public struct PersonaSheet: View {
    @Environment(\.look) private var look
    private let persona: Persona

    public var body: some View {
        let palette = look.palette
        ZStack(alignment: .top) {
            palette.background.opacity(0.4).ignoresSafeArea()
            VStack(spacing: 0) {
                Capsule().fill(palette.mute.opacity(0.5)).frame(width: 36, height: 5).padding(.top, 6)
                HStack {
                    RoundButton(symbol: "xmark", ascii: "x", size: 36)
                    Spacer()
                    Text(persona.name).ui(17, .semibold, look)
                    Spacer()
                    RoundButton(symbol: "checkmark", ascii: "ok", size: 36)
                }
                .padding(.horizontal, 16).padding(.vertical, 8)
                Scroller { sections.padding(.horizontal, 16).padding(.bottom, 30) }
            }
            .background(
                UnevenRoundedRectangle(topLeadingRadius: 34, topTrailingRadius: 34).fill(palette.background)
                    .overlay(
                        UnevenRoundedRectangle(topLeadingRadius: 34, topTrailingRadius: 34)
                            .fill(palette.card.opacity(0.35))
                    )
                    .ignoresSafeArea()
            )
            .padding(.top, 10)
        }
        .foregroundStyle(palette.foreground)
    }

    private var sections: some View {
        VStack(spacing: 18) {
            VStack(spacing: 8) {
                Avatar(persona: persona, size: 92)
                Text("Edit avatar").ui(14, .medium, look).foregroundStyle(look.palette.link)
            }
            Group2 {
                field("Name", persona.name)
                HStack {
                    Text("Agent").ui(16, .regular, look)
                    Spacer()
                    AgentChip(persona: persona, effort: true)
                }
                .padding(.horizontal, 14).frame(height: 48)
            }
            Group2(header: "Character") { paragraph(persona.api.character) }
            Group2(header: "Instructions") { paragraph(persona.api.instructions) }
            Group2(header: "Memory · 12 notes") { memory }
            Group2 {
                SettingRow(
                    symbol: "folder.fill", title: "Folder", tint: look.palette.amber, value: persona.api.folder,
                    divider: false
                )
            }
        }
    }

    @ViewBuilder private var memory: some View {
        let palette = look.palette
        ForEach(Fixtures.chiefMemory, id: \.key) { note in
            VStack(alignment: .leading, spacing: 2) {
                Text(note.key).ui(15, .regular, look).lineLimit(1)
                Text(note.value).ui(12, .regular, look).foregroundStyle(palette.mute)
            }
            .padding(.horizontal, 14).padding(.vertical, 9)
            .frame(maxWidth: .infinity, alignment: .leading)
            .overlay(alignment: .bottom) {
                Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, 14)
            }
        }
        HStack {
            Text("Show all 12").foregroundStyle(palette.link)
            Spacer()
            Text("Clear memory…").foregroundStyle(palette.red)
        }
        .ui(15, .regular, look).padding(.horizontal, 14).frame(height: 44)
    }

    public init(personaID: String = "chief") {
        persona = Fixtures.persona(personaID)
    }

    private func field(_ key: String, _ value: String) -> some View {
        HStack {
            Text(key).ui(16, .regular, look)
            Spacer()
            Text(value).ui(16, .regular, look).foregroundStyle(look.palette.mute)
        }
        .padding(.horizontal, 14).frame(height: 48)
        .overlay(alignment: .bottom) {
            Rectangle().fill(look.palette.line).frame(height: 0.5).padding(.leading, 14)
        }
    }

    private func paragraph(_ text: String) -> some View {
        Text(text).ui(15, .regular, look).padding(14).frame(maxWidth: .infinity, alignment: .leading)
            .fixedSize(horizontal: false, vertical: true)
    }
}
