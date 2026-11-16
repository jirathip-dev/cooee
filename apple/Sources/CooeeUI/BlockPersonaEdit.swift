import CooeeCore
import SwiftUI

internal struct PersonaActions {
    internal var close: () -> Void
    internal var save: () -> Void
    internal var clear: () -> Void
    internal var chat: () -> Void
    internal var agent: @MainActor @Sendable (String) -> Void = { _ in }
    internal var browse: () -> Void = {}

    internal init(
        close: @escaping () -> Void = {}, save: @escaping () -> Void = {}, clear: @escaping () -> Void = {},
        chat: @escaping () -> Void = {}
    ) {
        self.close = close
        self.save = save
        self.clear = clear
        self.chat = chat
    }
}

public struct PersonaEditBlocks: View {
    @Environment(\.look) private var look
    @Binding private var draft: APIPersona
    private let choices: PersonaChoices
    private let persona: Persona
    private let screen: JSONValue
    private let now: Date
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
        screen = .chiefEdit
        now = Date(timeIntervalSince1970: JSONValue.chiefEditNow)
        error = nil
        actions = PersonaActions()
    }

    internal init(
        draft: Binding<APIPersona>,
        choices: PersonaChoices,
        screen: JSONValue,
        error: String?,
        actions: PersonaActions
    ) {
        _draft = draft
        self.choices = choices
        persona = Persona(draft.wrappedValue)
        self.screen = screen
        now = Date()
        self.error = error
        self.actions = actions
    }

    private var memory: JSONValue? { screen.list("blocks").first { $0.type == "memory" } }

    private var memoryCount: Int { Int(memory?["count"]?.number ?? 0) }
}

extension PersonaEditBlocks {
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
                    style
                    Toggle("Pin to Home", isOn: $draft.pinned).ui(16, .regular, look).padding(.horizontal, 14)
                        .frame(height: 48)
                }
            }
            Group2(header: "Character") { paragraph("Character", $draft.character) }
            Group2(header: "Instructions") { paragraph("Instructions", $draft.instructions) }
            if !look.still {
                Button(action: actions.chat) {
                    Text(draft.id.isEmpty ? "Set up by chat instead" : "Refine by chat").ui(15, .medium, look)
                        .foregroundStyle(look.palette.link)
                }
                .buttonStyle(.plain)
            }
            if memoryCount > 0 { Group2(header: "Memory · \(memoryCount) notes") { notes } }
            Group2 {
                Button(action: actions.browse) {
                    SettingRow(
                        symbol: "folder.fill", title: "Folder", tint: look.palette.amber, value: draft.folder,
                        divider: !choices.machines.isEmpty
                    )
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain).accessibilityLabel("Folder")
                if !choices.machines.isEmpty { machine }
            }
            if let error { Text(error).ui(13, .regular, look).foregroundStyle(look.palette.red) }
        }
    }

    private var machine: some View {
        MachineMenu(
            machines: choices.machines, pick: { draft.machine = $0 },
            face: {
                HStack(spacing: 6) {
                    Text("Machine").ui(16, .regular, look)
                    Spacer()
                    if let row = choices.machines.first(where: { $0.name == draft.machine }) {
                        MachineDot(dot: row.dot)
                    }
                    Text(choices.machines.title(draft.machine)).ui(15, .regular, look).foregroundStyle(
                        look.palette.mute)
                }
                .padding(.horizontal, 14).frame(height: 48).contentShape(Rectangle())
            }
        )
        .buttonStyle(.plain).accessibilityLabel("Machine")
    }

    private var style: some View {
        HStack {
            Text("Style").ui(16, .regular, look)
            Spacer()
            Picker("Style", selection: $draft.style) {
                Text("Chat").tag(ChatStyle.chat)
                Text("Detailed").tag(ChatStyle.detailed)
            }
            .pickerStyle(.segmented).fixedSize()
        }
        .padding(.horizontal, 14).frame(height: 48)
        .overlay(alignment: .bottom) {
            Rectangle().fill(look.palette.line).frame(height: 0.5).padding(.leading, 14)
        }
    }

    @ViewBuilder private var notes: some View {
        let palette = look.palette
        let data = (try? JSONEncoder().encode(memory?["notes"] ?? .array([]))) ?? Data()
        let notes = Listing.memory(
            (try? JSONDecoder().decode([PersonaNote].self, from: data)) ?? [], persona: draft.name, now: now)
        ForEach(Array(notes.enumerated()), id: \.offset) { _, note in
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
            Text(notes.count < memoryCount ? "Show all \(memoryCount)" : "").foregroundStyle(palette.link)
            Spacer()
            if look.still {
                Text("Clear memory…").foregroundStyle(palette.red)
            } else {
                Menu("Clear memory…") {
                    Button("Clear all \(memoryCount) notes", role: .destructive, action: actions.clear)
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
