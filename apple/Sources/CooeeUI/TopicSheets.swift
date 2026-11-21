import ComposableArchitecture
import CooeeFeatures
import SwiftUI

extension ChatScreen {
    internal static func topics(_ store: StoreOf<ChatFeature>, _ bar: TopicBar) -> TopicActions {
        var topics = TopicActions(topics: bar.topics, current: store.current)
        (topics.pressed, topics.undo, topics.folder, topics.recent) = (bar.pressed, bar.undo, bar.folder, bar.recent)
        topics.open = { store.send(.topics(.open($0))) }
        topics.list = { store.send(.topics(.list(true))) }
        topics.press = { store.send(.topics(.press($0))) }
        topics.rename = { store.send(.topics(.rename($0))) }
        topics.archive = { store.send(.topics(.archive($0))) }
        topics.unarchive = { store.send(.topics(.undo)) }
        topics.pick = { store.send(.topics(.folder($0))) }
        topics.browse = { store.send(.topics(.browse(nil))) }
        return topics
    }
}

internal struct TopicSheets: ViewModifier {
    @Bindable internal var store: StoreOf<ChatFeature>

    internal func body(content: Content) -> some View {
        content
            .sheet(isPresented: Binding(get: { store.bar?.listing == true }, set: { store.send(.topics(.list($0))) })) {
                if let bar = store.bar {
                    TopicList(
                        persona: store.persona, topics: ChatScreen.topics(store, bar), renaming: bar.renaming,
                        search: Binding(get: { bar.search }, set: { store.send(.topics(.search($0))) }),
                        title: Binding(get: { store.bar?.title ?? "" }, set: { store.send(.topics(.title($0))) }),
                        retitle: { store.send(.topics(.retitle)) }, restore: { store.send(.topics(.restore($0))) }
                    )
                    .presentationBackground(.clear)
                }
            }
            .sheet(
                isPresented: Binding(
                    get: { store.bar?.folders != nil }, set: { _ in store.send(.topics(.browsed(nil))) })
            ) {
                if let listing = store.bar?.folders {
                    FolderBrowser(
                        listing: listing,
                        search: Binding(get: { store.bar?.find ?? "" }, set: { store.send(.topics(.find($0))) }),
                        browse: { store.send(.topics(.browse($0))) }, pick: { store.send(.topics(.folder($0))) }
                    )
                }
            }
    }
}
