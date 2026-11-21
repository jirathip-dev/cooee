import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import PhotosUI
import SwiftUI

internal struct PersonaScreen: View {
    @Bindable internal var store: StoreOf<PersonaFeature>
    internal let sprite: (String?) -> Void

    private var choices: PersonaChoices {
        var choices = PersonaChoices(
            agents: store.agents, models: store.models, efforts: store.efforts, machines: store.machines,
            connectors: store.connectors
        )
        choices.lists = store.lists
        choices.rules = store.isNew ? nil : store.rules.rules.count
        return choices
    }

    internal var body: some View {
        PersonaEditBlocks(
            draft: $store.draft,
            choices: choices,
            screen: store.screen ?? .bundledPersona,
            error: store.error,
            actions: actions
        )
        .disabled(store.busy)
        .task { await store.send(.task).finish() }
        .task { if !store.isNew { await store.send(.rules(.task)).finish() } }
        .sheet(isPresented: Binding(get: { store.folders != nil }, set: { _ in store.send(.folder(nil)) })) {
            if let listing = store.folders {
                FolderBrowser(
                    listing: listing, search: $store.search, browse: { store.send(.browse($0)) },
                    pick: { store.send(.folder($0)) }
                )
            }
        }
        .sheet(isPresented: $store.showingAll) {
            AllNotes(
                notes: PersonaEditBlocks.notes(store.screen ?? .bundledPersona, persona: store.draft.name, now: Date())
            ) { store.showingAll = false }
        }
        .alert($store.scope(state: \.alert, action: \.alert))
        .sheet(isPresented: $store.rules.showing) {
            RulesScreen(
                store: store.scope(state: \.rules, action: \.rules), name: store.draft.name,
                folder: ["", "scratch"].contains(store.draft.folder) ? nil : store.draft.folder,
                agent: store.agents.first { $0.id == store.draft.agent })
        }
        .photosPicker(isPresented: $store.choosingPhoto, selection: photo, matching: .images)
    }

    private var photo: Binding<PhotosPickerItem?> {
        Binding(
            get: { nil },
            set: { item in
                Task {
                    guard let data = try? await item?.loadTransferable(type: Data.self) else { return }
                    store.draft.avatar = AvatarPicture.dataURL(data) ?? store.draft.avatar
                }
            }
        )
    }

    private var actions: PersonaActions {
        var actions = PersonaActions(
            close: { store.send(.close) }, save: { store.send(.save) }, clear: { store.send(.clearMemory) },
            chat: { store.send(.chat) }
        )
        actions.agent = { store.send(.agent($0)) }
        actions.browse = { store.send(.browse(nil)) }
        actions.more = { store.send(.showAll) }
        actions.delete = store.isNew || store.draft.role == "chief" ? nil : { store.send(.delete) }
        actions.photo = { store.choosingPhoto = true }
        actions.sprite = sprite
        actions.skill = { store.send(.addSkill($0)) }
        actions.catalogue = { store.send(.delegate(.connectors)) }
        actions.rules = { store.send(.rules(.open)) }
        return actions
    }
}

internal struct AllNotes: View {
    internal let notes: [Field]
    internal let done: () -> Void

    internal var body: some View {
        NavigationStack {
            List(Array(notes.enumerated()), id: \.offset) { _, note in
                VStack(alignment: .leading, spacing: 2) {
                    Text(note.key)
                    Text(note.value).font(.caption).foregroundStyle(.secondary)
                }
            }
            .navigationTitle("Memory · \(notes.count) notes")
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done", action: done) } }
        }
    }
}
