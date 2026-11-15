import ComposableArchitecture
import CooeeFeatures
import SwiftUI

internal struct ChatScreen: View {
    @Bindable internal var store: StoreOf<ChatFeature>
    internal let back: Bool
    internal var headed = true
    internal let onBack: () -> Void

    internal static func actions(_ store: StoreOf<ChatFeature>, back onBack: @escaping () -> Void) -> ChatActions {
        var actions = ChatActions(
            draft: Binding(get: { store.draft }, set: { store.send(.draftChanged($0)) }),
            files: store.files.map(\.name),
            send: { store.send(.send) },
            attach: { store.send(.attach($0)) },
            detach: { store.send(.detach($0)) },
            stop: { store.send(.stop) },
            steer: { store.send(.steer($0)) },
            unqueue: { store.send(.unqueue($0)) },
            answer: { store.send(.answer($0, $1)) },
            answering: store.answering,
            back: onBack
        )
        actions.retry = { store.send(.retry($0)) }
        let (places, attachable) = (store.places, store.attachable)
        actions.places = places.map(\.label)
        actions.open = { store.send(.plugins(.open(places[$0]))) }
        actions.attachable = attachable.map(\.name)
        actions.attachPlugin = { store.send(.plugins(.attach(attachable[$0]))) }
        return actions
    }

    internal var body: some View {
        ChatView(
            persona: store.persona,
            items: store.drawn,
            working: store.working,
            since: store.since,
            queued: store.queue.map(\.text),
            back: back,
            headed: headed,
            actions: Self.actions(store, back: onBack)
        )
        .task { await store.send(.task).finish() }
        .task { await store.send(.plugins(.load)).finish() }
        .sheet(item: $store.scope(state: \.screen, action: \.screen)) { PluginScreenSheet(store: $0) }
        .sheet(item: $store.scope(state: \.attaching, action: \.attaching)) { AttachSheet(store: $0) }
    }
}
