import CooeeCore
import SwiftUI

public struct PersonaActions {
    public var close: () -> Void
    public var save: () -> Void
    public var clear: () -> Void
    public var agent: @MainActor @Sendable (String) -> Void = { _ in }
    public var browse: () -> Void = {}

    public init(close: @escaping () -> Void = {}, save: @escaping () -> Void = {}, clear: @escaping () -> Void = {}) {
        self.close = close
        self.save = save
        self.clear = clear
    }
}

public struct PersonaMemory {
    public let notes: [Field]
    public let count: Int
    public var more = ""

    public init(notes: [Field], count: Int, more: String = "") {
        self.notes = notes
        self.count = count
        self.more = more
    }
}

public struct PersonaSheet: View {
    @Environment(\.look) private var look
    @Binding private var draft: APIPersona
    private let choices: PersonaChoices
    private let persona: Persona
    private let memory: PersonaMemory
    private let error: String?
    private let actions: PersonaActions

    public var body: some View {
        let palette = look.palette
        ZStack(alignment: .top) {
            palette.background.opacity(0.4).ignoresSafeArea()
            VStack(spacing: 0) {
                Capsule().fill(palette.mute.opacity(0.5)).frame(width: 36, height: 5).padding(.top, 6)
                HStack {
                    Button(action: actions.close) { RoundButton(symbol: "xmark", ascii: "x", size: 36) }
                        .buttonStyle(.plain).accessibilityLabel("Close")
                    Spacer()
                    Text(draft.name.isEmpty ? "New persona" : draft.name).ui(17, .semibold, look)
                    Spacer()
                    Button(action: actions.save) { RoundButton(symbol: "checkmark", ascii: "ok", size: 36) }
                        .buttonStyle(.plain).accessibilityLabel("Save")
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

    public init(personaID: String = "chief") {
        persona = Fixtures.persona(personaID)
        _draft = .constant(persona.api)
        choices = PersonaChoices()
        memory = PersonaMemory(notes: Fixtures.chiefMemory, count: 12, more: "Show all 12")
        error = nil
        actions = PersonaActions()
    }

    public init(
        draft: Binding<APIPersona>,
        choices: PersonaChoices,
        memory: PersonaMemory,
        error: String?,
        actions: PersonaActions
    ) {
        _draft = draft
        self.choices = choices
        persona = Persona(draft.wrappedValue)
        self.memory = memory
        self.error = error
        self.actions = actions
    }
}

extension PersonaSheet {
    private var sections: some View {
        VStack(spacing: 18) {
            VStack(spacing: 8) {
                Avatar(persona: persona, size: 92)
                Text("Edit avatar").ui(14, .medium, look).foregroundStyle(look.palette.link)
            }
            Group2 {
                field("Name", $draft.name)
                HStack {
                    Text("Agent").ui(16, .regular, look)
                    Spacer()
                    AgentMenu(draft: $draft, choices: choices, pick: actions.agent)
                }
                .padding(.horizontal, 14).frame(height: 48)
                if !look.still {
                    Toggle("Pin to Home", isOn: $draft.pinned).ui(16, .regular, look).padding(.horizontal, 14)
                        .frame(height: 48)
                }
            }
            Group2(header: "Character") { paragraph("Character", $draft.character) }
            Group2(header: "Instructions") { paragraph("Instructions", $draft.instructions) }
            if memory.count > 0 { Group2(header: "Memory · \(memory.count) notes") { notes } }
            Group2 {
                Button(action: actions.browse) {
                    SettingRow(
                        symbol: "folder.fill", title: "Folder", tint: look.palette.amber, value: draft.folder,
                        divider: false
                    )
                }
                .buttonStyle(.plain).accessibilityLabel("Folder")
            }
            if let error { Text(error).ui(13, .regular, look).foregroundStyle(look.palette.red) }
        }
    }

    @ViewBuilder private var notes: some View {
        let palette = look.palette
        ForEach(Array(memory.notes.enumerated()), id: \.offset) { _, note in
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
            Text(memory.more).foregroundStyle(palette.link)
            Spacer()
            if look.still {
                Text("Clear memory…").foregroundStyle(palette.red)
            } else {
                Menu("Clear memory…") {
                    Button("Clear all \(memory.count) notes", role: .destructive, action: actions.clear)
                }
                .foregroundStyle(palette.red)
            }
        }
        .ui(15, .regular, look).padding(.horizontal, 14).frame(height: 44)
    }

    private func field(_ key: String, _ value: Binding<String>) -> some View {
        HStack {
            Text(key).ui(16, .regular, look)
            Spacer()
            if look.still {
                Text(value.wrappedValue).ui(16, .regular, look).foregroundStyle(look.palette.mute)
            } else {
                TextField(key, text: value).multilineTextAlignment(.trailing).ui(16, .regular, look)
                    .foregroundStyle(look.palette.mute)
            }
        }
        .padding(.horizontal, 14).frame(height: 48)
        .overlay(alignment: .bottom) {
            Rectangle().fill(look.palette.line).frame(height: 0.5).padding(.leading, 14)
        }
    }

    @ViewBuilder private func paragraph(_ key: String, _ text: Binding<String>) -> some View {
        if look.still {
            Text(text.wrappedValue).ui(15, .regular, look).padding(14).frame(maxWidth: .infinity, alignment: .leading)
                .fixedSize(horizontal: false, vertical: true)
        } else {
            TextField(key, text: text, axis: .vertical).ui(15, .regular, look).padding(14)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}
