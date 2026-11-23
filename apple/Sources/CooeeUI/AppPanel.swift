import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

/// The menu bar panel on the app's store (#659): a row opens its chat, then `front` brings the main window forward.
public struct AppPanel: View {
    @Environment(\.colorScheme) private var scheme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private let store: StoreOf<AppFeature>
    private let front: () -> Void
    private let quit: () -> Void

    public var body: some View {
        TimelineView(.periodic(from: .now, by: 1)) { context in
            MenuPanel(store.state.glance(now: context.date)) { tap in
                switch tap {
                case .chat(let id):
                    store.send(.open(id))
                    front()
                case .main: front()
                case .retry: refresh()
                case .quit: quit()
                }
            }
        }
        .environment(\.look, store.menu.look(dark: scheme == .dark, reduceMotion: reduceMotion))
        .task { refresh() }
    }

    private func refresh() {
        if store.home != nil { store.send(.home(.refresh)) }
    }

    public init(store: StoreOf<AppFeature>, front: @escaping () -> Void, quit: @escaping () -> Void) {
        self.store = store
        self.front = front
        self.quit = quit
    }
}
