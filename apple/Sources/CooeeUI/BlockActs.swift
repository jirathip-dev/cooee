import CooeeCore
import SwiftUI

internal struct BlockActs: ViewModifier {
    @Environment(\.act) private var act
    @Environment(\.swiped) private var swiped
    @State private var pending: JSONValue?
    @State private var open: Bool?
    internal let block: JSONValue

    private var swipe: [JSONValue] { block.list("swipe") }
    private var shown: Bool { !swipe.isEmpty && (open ?? (swiped == block.id)) }

    private var menu: [JSONValue] {
        #if os(iOS)
            block.list("menu")
        #else
            block.list("menu") + swipe
        #endif
    }

    internal func body(content: Content) -> some View {
        swiping(content)
            .contextMenu { MenuItems(acts: menu, run: ask) }
            .modifier(Confirming(pending: $pending, run: run))
    }

    @ViewBuilder
    private func swiping(_ content: Content) -> some View {
        if swipe.isEmpty {
            content
        } else {
            content.offset(x: shown ? -CGFloat(swipe.count) * SwipeButtons.width : 0)
                .overlay(alignment: .trailing) {
                    if shown {
                        SwipeButtons(acts: swipe) { item in
                            open = false
                            ask(item)
                        }
                    }
                }
                .clipped()
                #if os(iOS)
                    .simultaneousGesture(drag)
                #endif
                .animation(.snappy, value: shown)
        }
    }

    #if os(iOS)
        private var drag: some Gesture {
            DragGesture(minimumDistance: 24).onEnded { drag in
                let (across, down) = (drag.translation.width, drag.translation.height)
                if abs(across) > abs(down) { open = across < 0 }
            }
        }
    #endif

    private func ask(_ item: JSONValue) {
        if Confirm(for: item) == nil { run(item) } else { pending = item }
    }

    private func run(_ item: JSONValue) {
        if let action = BlockAction(action: nil, source: item, values: [:]) { act(action) }
    }
}

internal struct MenuItems: View {
    internal let acts: [JSONValue]
    internal let run: (JSONValue) -> Void

    internal var body: some View {
        ForEach(Array(acts.enumerated()), id: \.offset) { _, item in
            Button(item.string("label") ?? "", role: item.string("tone") == "bad" ? .destructive : nil) { run(item) }
        }
    }
}

private struct SwipeButtons: View {
    fileprivate static let width: CGFloat = 76
    @Environment(\.look) private var look
    fileprivate let acts: [JSONValue]
    fileprivate let run: (JSONValue) -> Void

    fileprivate var body: some View {
        HStack(spacing: 0) {
            ForEach(Array(acts.enumerated()), id: \.offset) { _, item in
                Button {
                    run(item)
                } label: {
                    Text(item.string("label") ?? "").ui(13, .semibold, look).foregroundStyle(look.palette.background)
                        .frame(width: Self.width).frame(maxHeight: .infinity)
                        .background(look.palette.tone(item.string("tone")) ?? look.palette.link)
                }
                .buttonStyle(.plain)
            }
        }
    }
}

internal struct Entering: ViewModifier {
    internal let transition: LayoutTransition
    internal let shown: Double

    internal func body(content: Content) -> some View {
        content.opacity(transition == .fade ? shown : 1)
            .visualEffect { [transition, shown] effect, proxy in
                effect.offset(
                    x: transition == .push ? (1 - shown) * proxy.size.width : 0,
                    y: transition == .sheet ? (1 - shown) * proxy.size.height : 0)
            }
    }
}

extension LayoutTransition {
    internal var entering: AnyTransition {
        .modifier(active: Entering(transition: self, shown: 0), identity: Entering(transition: self, shown: 1))
    }
}

private struct SwipedKey: EnvironmentKey {
    fileprivate static var defaultValue: String? { nil }
}

private struct RefreshKey: EnvironmentKey {
    fileprivate static var defaultValue: (@MainActor () -> Void)? { nil }
}

extension EnvironmentValues {
    internal var swiped: String? {
        get { self[SwipedKey.self] }
        set { self[SwipedKey.self] = newValue }
    }

    internal var refresh: (@MainActor () -> Void)? {
        get { self[RefreshKey.self] }
        set { self[RefreshKey.self] = newValue }
    }
}

internal struct Refreshing: ViewModifier {
    internal let refresh: (@MainActor () -> Void)?

    @ViewBuilder
    internal func body(content: Content) -> some View {
        if let refresh { content.refreshable { refresh() } } else { content }
    }
}
