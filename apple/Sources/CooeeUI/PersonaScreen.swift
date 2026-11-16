import ComposableArchitecture
import CooeeFeatures
import SwiftUI

internal struct PersonaScreen: View {
    @Bindable internal var store: StoreOf<PersonaFeature>

    internal var body: some View {
        PersonaEditBlocks(
            draft: $store.draft,
            choices: PersonaChoices(
                agents: store.agents, models: store.models, efforts: store.efforts, machines: store.machines
            ),
            screen: store.screen ?? .bundledPersona,
            error: store.error,
            actions: actions
        )
        .disabled(store.busy)
        .task { await store.send(.task).finish() }
        .sheet(isPresented: Binding(get: { store.folders != nil }, set: { _ in store.send(.folder(nil)) })) {
            if let listing = store.folders {
                FolderBrowser(
                    listing: listing, search: $store.search, browse: { store.send(.browse($0)) },
                    pick: { store.send(.folder($0)) }
                )
            }
        }
    }

    private var actions: PersonaActions {
        var actions = PersonaActions(
            close: { store.send(.close) }, save: { store.send(.save) }, clear: { store.send(.clearMemory) }
        )
        actions.agent = { store.send(.agent($0)) }
        actions.browse = { store.send(.browse(nil)) }
        return actions
    }
}
