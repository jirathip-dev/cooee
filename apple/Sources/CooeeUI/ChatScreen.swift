import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct ChatScreen: View {
    @Bindable internal var store: StoreOf<ChatFeature>
    internal let back: Bool
    internal var headed = true
    internal var backTo: String?
    internal let onBack: () -> Void

    internal static func actions(_ store: StoreOf<ChatFeature>, back onBack: @escaping () -> Void) -> ChatActions {
        var actions = ChatActions(
            draft: Binding(get: { store.draft }, set: { store.send(.compose(.draftChanged($0))) }),
            files: store.files.map(\.name),
            send: { store.send(.compose(.send)) },
            attach: { store.send(.compose(.attach($0))) },
            detach: { store.send(.compose(.detach($0))) },
            stop: { store.send(.compose(.stop)) },
            steer: { store.send(.compose(.steer($0))) },
            unqueue: { store.send(.compose(.unqueue($0))) },
            answer: { store.send(.cards(.answer($0, $1))) },
            answering: store.answering,
            back: onBack
        )
        actions.retry = { store.send(.outbox(.retry($0))) }
        let (places, attachable) = (store.places, store.attachable)
        actions.places = places.map(\.label)
        actions.open = { store.send(.plugins(.open(places[$0]))) }
        actions.attachable = attachable.map(\.name)
        actions.attachPlugin = { store.send(.plugins(.attach(attachable[$0]))) }
        actions.workflow = store.workflow.map { Workflow.issues($0).count(where: \.needsYou) }
        actions.showWorkflow = { store.send(.workflow(.show(true))) }
        actions.workflowOpen = store.workflowOpen
        actions.step = store.step
        return actions
    }

    private var actions: ChatActions {
        var actions = Self.actions(store, back: onBack)
        actions.backTo = backTo
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
            actions: actions
        )
        .task { await store.send(.task).finish() }
        .task { await store.send(.plugins(.load)).finish() }
        .sheet(item: $store.scope(state: \.screen, action: \.screen)) { PluginScreenSheet(store: $0) }
        .sheet(item: $store.scope(state: \.attaching, action: \.attaching)) { AttachSheet(store: $0) }
        .sheet(isPresented: Binding(get: { store.workflowOpen }, set: { store.send(.workflow(.show($0))) })) {
            WorkflowSheet(
                issues: Workflow.issues(store.workflow ?? []),
                unfolded: store.unfolded,
                unfold: { store.send(.workflow(.unfold($0))) },
                open: { store.send(.workflow(.open($0))) },
                done: { store.send(.workflow(.show(false))) }
            )
            .presentationDetents([.fraction(0.8)])
        }
    }
}
