import CooeeCore
import SwiftUI

public struct PersonaChoices {
    public var agents: [AgentOption]
    public var models: [String]
    public var efforts: [String]

    public init(agents: [AgentOption] = [], models: [String] = [], efforts: [String] = []) {
        self.agents = agents
        self.models = models
        self.efforts = efforts
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
