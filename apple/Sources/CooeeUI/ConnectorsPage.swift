import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct ConnectorsPage: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<ConnectorsFeature>

    internal var body: some View {
        ConnectorsScroll(store: store) { CatalogueGroups(list: store.list, actions: .of(store)) }
            .navigationTitle("Connectors")
            .navigationDestination(item: Binding(get: { store.entry }, set: { store.send(.open($0)) })) { name in
                ConnectorsScroll(store: store) {
                    if let connector = store.list.named(name) {
                        let pending = store.signingIn.flatMap { $0.server == name ? $0.account : nil }
                        EntryBody(connector: pending.map(connector.signingIn) ?? connector, actions: .of(store))
                    }
                }
                .navigationTitle(store.list.named(name)?.label ?? name)
            }
    }
}

internal struct ConnectorsScroll<Content: View>: View {
    @Environment(\.look) private var look
    @Bindable internal var store: StoreOf<ConnectorsFeature>
    @ViewBuilder internal let content: Content

    internal var body: some View {
        ZStack(alignment: .bottom) {
            ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    content
                    if let error = store.error { Text(error).ui(13, .regular, look).foregroundStyle(look.palette.red) }
                }
                .padding(16)
            }
            if store.connected != nil || store.token != nil { Dim().onTapGesture { store.send(.notNow) } }
            if let flow = store.connected {
                ConnectedSheet(
                    label: store.list.named(flow.connector)?.label ?? flow.connector, account: flow.account,
                    personas: store.personas.map { Persona($0) }, attach: { store.send(.attach(persona: $0)) },
                    notNow: { store.send(.notNow) })
            }
            if let token = store.token {
                TokenSheet(
                    title:
                        "Sign in to \(store.list.named(token.connector)?.label ?? token.connector) · \(token.account)",
                    help: token.help, value: Binding(get: { token.value }, set: { store.send(.typed($0)) }),
                    save: { store.send(.paste) }, close: { store.send(.notNow) })
            }
        }
        .background(Backdrop()).foregroundStyle(look.palette.foreground)
    }
}

extension ConnectorActions {
    @MainActor
    internal static func of(_ store: StoreOf<ConnectorsFeature>) -> Self {
        ConnectorActions(
            open: { store.send(.open($0)) }, connect: { store.send(.connect($0, account: $1)) },
            signOut: { store.send(.signOut($0, account: $1)) }, remove: { store.send(.remove($0)) })
    }
}
