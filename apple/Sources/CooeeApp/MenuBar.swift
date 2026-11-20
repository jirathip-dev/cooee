#if os(macOS)
    import AppKit
    import ComposableArchitecture
    import CooeeFeatures
    import CooeeUI
    import SwiftUI

    /// The menu bar extra (#659): the template mark with the ask count, and the shared panel on the app's store.
    public struct CooeeMenuBar: Scene {
        public static let window = "main"

        private let store = CooeeRoot.live

        public var body: some Scene {
            MenuBarExtra {
                Shell(store: store)
            } label: {
                MenuBarLabel(store: store)
            }
            .menuBarExtraStyle(.window)
        }

        public init() {}
    }

    private struct Shell: View {
        @Environment(\.openWindow) private var openWindow
        @Environment(\.dismiss) private var dismiss
        fileprivate let store: StoreOf<AppFeature>

        fileprivate var body: some View {
            AppPanel(store: store, front: front) { NSApp.terminate(nil) }
        }

        private func front() {
            dismiss()
            NSApp.activate()
            let main = NSApp.windows.first {
                $0.identifier?.rawValue.hasPrefix(CooeeMenuBar.window) == true && ($0.isVisible || $0.isMiniaturized)
            }
            guard let main else { return openWindow(id: CooeeMenuBar.window) }
            main.deminiaturize(nil)
            main.makeKeyAndOrderFront(nil)
        }
    }

    private struct MenuBarLabel: View {
        fileprivate let store: StoreOf<AppFeature>

        fileprivate var body: some View {
            let glance = store.state.glance(now: .now)
            switch glance.link {
            case .unreachable: Label("!", image: "MenuBarMark").labelStyle(.titleAndIcon)
            case .running where !glance.asks.isEmpty:
                Label("\(glance.asks.count)", image: "MenuBarMark").labelStyle(.titleAndIcon)
            case .running, .unpaired: Image("MenuBarMark")
            }
        }
    }
#endif
