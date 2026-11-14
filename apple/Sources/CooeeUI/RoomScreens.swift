import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct RoomScreen: View {
    internal let store: StoreOf<ChatFeature>
    internal let back: Bool
    internal let persona: (String) -> Void
    internal let onBack: () -> Void

    private var scene: RoomScene? {
        guard let room = store.room else { return nil }
        let cast = room.cast.map { Persona($0) }
        let members = room.members.map { id in
            cast.first { $0.id == id } ?? Persona(APIPersona(id: id, name: id, agent: ""))
        }
        return RoomScene(
            title: room.title, members: members, others: cast.filter { !room.members.contains($0.id) },
            round: room.round, items: store.drawn, inspecting: room.inspecting
        )
    }

    private var actions: RoomActions {
        RoomActions(
            chat: ChatActions(
                draft: Binding(get: { store.draft }, set: { store.send(.draftChanged($0)) }),
                files: store.files.map(\.name),
                send: { store.send(.send) },
                attach: { store.send(.attach($0)) },
                detach: { store.send(.detach($0)) },
                stop: { store.send(.stop) },
                back: onBack
            ),
            inspect: { store.send(.inspect($0)) },
            move: { store.send(.move($0, $1)) },
            add: { store.send(.seat($0)) },
            remove: { store.send(.unseat($0)) },
            persona: persona
        )
    }

    internal var body: some View {
        if let scene {
            RoomView(scene: scene, back: back, wide: !back, actions: actions)
                .sheet(
                    isPresented: Binding(
                        get: { scene.inspecting != nil }, set: { if !$0 { store.send(.inspect(nil)) } })
                ) {
                    if let member = scene.members.first(where: { $0.id == scene.inspecting }) {
                        MemberSheet(scene: scene, member: member, actions: actions).presentationDetents([
                            .medium, .large,
                        ])
                    }
                }
                .task { await store.send(.task).finish() }
        }
    }
}

internal struct NewRoomScreen: View {
    internal let store: StoreOf<NewRoomFeature>

    internal var body: some View {
        NewRoomSheet(
            personas: store.personas.map { Persona($0) },
            title: store.title,
            picked: store.picked,
            error: store.error,
            actions: NewRoomActions(
                title: Binding(get: { store.title }, set: { store.send(.title($0)) }),
                toggle: { store.send(.toggle($0)) },
                move: { store.send(.move($0, $1)) },
                create: { store.send(.create) },
                cancel: { store.send(.delegate(.cancel)) }
            )
        )
        .presentationDetents([.large])
    }
}
