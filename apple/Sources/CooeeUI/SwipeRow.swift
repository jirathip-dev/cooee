import SwiftUI

internal struct SwipeAction: Identifiable {
    internal let symbol: String
    internal let title: String
    internal var amber = false
    internal let run: () -> Void

    internal var id: String { title }
}

internal struct SwipeRow<Content: View>: View {
    @Environment(\.look) private var look
    private let swiped: State<Bool>
    private let actions: [SwipeAction]
    private let divider: Bool
    private let content: Content

    private var width: CGFloat { CGFloat(actions.count) * 75 }

    internal var body: some View {
        let palette = look.palette
        ZStack(alignment: .trailing) {
            content.offset(x: swiped.wrappedValue ? -width : 0)
            if swiped.wrappedValue {
                HStack(spacing: 0) { ForEach(actions) { button($0) } }
            }
        }
        .frame(height: swiped.wrappedValue ? 58 : nil).clipped()
        .overlay(alignment: .bottom) {
            if divider { Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, 48) }
        }
        .gesture(
            DragGesture(minimumDistance: 20).onEnded { drag in
                withAnimation(.snappy) { swiped.wrappedValue = drag.translation.width < -40 }
            }
        )
    }

    internal init(actions: [SwipeAction], open: Bool, divider: Bool, @ViewBuilder content: () -> Content) {
        self.actions = actions
        swiped = State(initialValue: open)
        self.divider = divider
        self.content = content()
    }

    private func button(_ action: SwipeAction) -> some View {
        let tint = action.amber ? look.palette.amber : look.palette.link
        return Button {
            swiped.wrappedValue = false
            action.run()
        } label: {
            VStack(spacing: 3) {
                Glyph(symbol: action.symbol, ascii: "[\(action.title.prefix(1).lowercased())]", size: 15)
                Text(action.title).ui(11, .semibold, look)
            }
            .foregroundStyle(look.outline ? tint : (look.palette.dark ? Color.black : .white))
            .frame(width: 75, height: 58).background(look.outline ? Color.clear : tint)
        }
        .buttonStyle(.plain)
    }
}
