import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

extension ChatScreen {
    internal static func voice(_ store: StoreOf<ChatFeature>) -> VoiceActs {
        var voice = VoiceActs(dictation: store.dictation)
        voice.mic = { store.send(.voice(.mic)) }
        voice.hold = { store.send(.voice(.hold($0))) }
        voice.pick = { store.send(.voice(.pick($0))) }
        if case .thread = store.target, store.room == nil, !store.persona.api.id.isEmpty {
            voice.call = { store.send(.call(.start)) }
        }
        return voice
    }
}

internal struct CallCover: ViewModifier {
    internal let store: StoreOf<ChatFeature>

    private var shown: Binding<Bool> {
        Binding(get: { store.call?.shown == true }, set: { store.send(.call(.shown($0))) })
    }

    internal func body(content: Content) -> some View {
        #if os(iOS)
            content.fullScreenCover(isPresented: shown) { screen }
        #else
            content.sheet(isPresented: shown) { screen.frame(minWidth: 402, minHeight: 760) }
        #endif
    }

    @ViewBuilder private var screen: some View {
        if let call = store.call {
            let items = store.items.filter { $0.id > call.from }
            let tool = items.last { $0.kind == "tool" && !["completed", "failed"].contains($0.status ?? "") }
            TimelineView(.periodic(from: call.started, by: 1)) { context in
                CallScreen(
                    persona: store.persona, title: store.bar?.topics.row(store.current)?.title ?? "",
                    seconds: Int(context.date.timeIntervalSince(call.started)),
                    lines: Call.parts(items).last?.call?.lines ?? [], speaking: call.speaking, tool: tool?.title,
                    acts: acts(call))
            }
        }
    }

    private func acts(_ call: Calling) -> CallActs {
        var acts = CallActs(muted: call.muted, speaker: call.speaker)
        acts.mute = { store.send(.call(.mute)) }
        acts.loud = { store.send(.call(.speaker)) }
        acts.thread = { store.send(.call(.shown(false))) }
        acts.end = { store.send(.call(.end)) }
        return acts
    }
}
