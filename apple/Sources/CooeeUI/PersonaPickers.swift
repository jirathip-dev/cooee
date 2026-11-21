import CooeeCore
import SwiftUI

internal struct PersonaActions {
    internal var close: () -> Void
    internal var save: () -> Void
    internal var clear: () -> Void
    internal var chat: () -> Void
    internal var agent: @MainActor @Sendable (String) -> Void = { _ in }
    internal var browse: () -> Void = {}
    internal var more: () -> Void = {}
    internal var delete: (() -> Void)?
    internal var photo: () -> Void = {}
    internal var sprite: (String?) -> Void = { _ in }
    internal var skill: (String) -> Void = { _ in }
    internal var catalogue: () -> Void = {}
    internal var rules: () -> Void = {}

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

internal struct PersonaChoices {
    internal var agents: [AgentOption]
    internal var models: [String]
    internal var efforts: [String]
    internal var machines: [MachineRow]
    internal var connectors: ConnectorList
    internal var lists: [ModelList] = []
    internal var rules: Int?

    internal init(
        agents: [AgentOption] = [],
        models: [String] = [],
        efforts: [String] = [],
        machines: [MachineRow] = [],
        connectors: ConnectorList = ConnectorList()
    ) {
        self.agents = agents
        self.models = models
        self.efforts = efforts
        self.machines = machines
        self.connectors = connectors
    }
}

internal struct AgentMenu: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal let choices: PersonaChoices
    internal var effort = true
    internal let pick: @MainActor @Sendable (String) -> Void
    internal let set: (String, String) -> Void

    internal var body: some View {
        if look.still {
            AgentChip(persona: persona, effort: effort)
        } else {
            let api = persona.api
            Menu {
                if choices.agents.isEmpty { Text("No agents found: run cooee doctor on the Mac") }
                Picker("Agent", selection: Binding(get: { api.agent }, set: { pick($0) })) {
                    ForEach(choices.agents) { Text($0.id).tag($0.id) }
                }
                .pickerStyle(.inline)
                list("Model", Binding(get: { api.model }, set: { set($0, api.effort) }), choices.models)
                if !choices.efforts.isEmpty {
                    list("Effort", Binding(get: { api.effort }, set: { set(api.model, $0) }), choices.efforts)
                }
            } label: {
                AgentChip(persona: persona, effort: effort)
            }
            .accessibilityLabel("Agent")
        }
    }

    private func list(_ title: String, _ value: Binding<String>, _ options: [String]) -> some View {
        Picker(title, selection: value) {
            Text(options.isEmpty ? "Default: none seen yet, start a chat to list them" : "Default").tag("")
            ForEach(options, id: \.self) { Text($0).tag($0) }
        }
        .pickerStyle(.inline)
    }
}

internal struct FolderBrowser: View {
    internal let listing: FolderListing
    @Binding internal var search: String
    internal let browse: (String?) -> Void
    internal let pick: (String?) -> Void

    internal var body: some View {
        let shown = listing.shown(search)
        NavigationStack {
            List {
                if listing.top {
                    Button("scratch: each chat its own folder") { pick("scratch") }
                } else {
                    Button("Use \(name(listing.path))") { pick(listing.path) }
                }
                ForEach(shown, id: \.self) { folder in
                    Button {
                        browse(folder)
                    } label: {
                        Label(name(folder), systemImage: "folder")
                    }
                }
                if shown.isEmpty {
                    Text(listing.top ? "No roots: add a folder on the Mac" : "No sub-folders here").foregroundStyle(
                        .secondary)
                }
            }
            .searchable(text: $search)
            .navigationTitle(listing.top ? "Folder" : name(listing.path))
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(listing.top ? "Cancel" : "Back") { listing.top ? pick(nil) : browse(listing.parent) }
                }
            }
        }
    }

    private func name(_ path: String) -> String { (path as NSString).lastPathComponent }
}

internal struct AvatarEditor: View {
    @Environment(\.look) private var look
    @Binding internal var avatar: String?
    internal let photo: () -> Void

    internal var body: some View {
        let label = Text("Edit avatar").ui(14, .medium, look).foregroundStyle(look.palette.link)
        if look.still {
            label
        } else {
            Menu {
                Button("Choose photo…", systemImage: "photo", action: photo)
                Picker("Initials and colour", selection: $avatar) {
                    ForEach(Tint.allCases, id: \.self) { tint in
                        Label(tint.rawValue.capitalized, systemImage: "circle.fill")
                            .foregroundStyle(look.palette.tint(tint)).tag(Optional(tint.avatar))
                    }
                }
                .pickerStyle(.inline)
                if avatar != nil { Button("Default avatar", role: .destructive) { avatar = nil } }
            } label: {
                label
            }
            .accessibilityLabel("Edit avatar")
        }
    }
}

extension PersonaEditBlocks {
    internal static func notes(_ screen: JSONValue, persona: String, now: Date) -> [Field] {
        let memory = screen.list("blocks").first { $0.type == "memory" }
        let data = (try? JSONEncoder().encode(memory?["notes"] ?? .array([]))) ?? Data()
        return Listing.memory(
            (try? JSONDecoder().decode([PersonaNote].self, from: data)) ?? [], persona: persona, now: now)
    }
}
