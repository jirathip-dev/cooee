import CooeeCore
import SwiftUI

/// The Mac's sidebar component: the home column, any screens the layout places in it, and the detail beside it.
public struct Sidebar<Column: View, Detail: View>: View {
    @Environment(\.look) private var look
    private let screen: HomeScreen
    private let pins: [Pin]
    private let all: [Persona]
    private let chats: [ListedChat]
    private let selected: String?
    private let actions: HomeActions
    private let column: Column
    private let detail: Detail

    public var body: some View {
        let palette = look.palette
        HStack(spacing: 0) {
            VStack(spacing: 0) {
                TopBar(title: screen.title, personas: all, actions: actions).padding(.top, look.still ? 34 : 8)
                Scroller {
                    HomeBlocks(
                        screen: screen, pins: pins, chats: chats, selected: selected, actions: actions, wide: true
                    )
                    if !actions.others.isEmpty { OthersRow(titles: actions.others, open: actions.openOthers) }
                    column
                }
            }
            .frame(width: 360).background(palette.card.opacity(look.glass ? 0.45 : 0.8))
            .overlay(alignment: .trailing) { Rectangle().fill(palette.line).frame(width: 1) }
            detail.padding(.top, 12).frame(maxWidth: .infinity)
        }
        .foregroundStyle(palette.foreground)
    }

    public init(
        screen: HomeScreen = .bundled,
        pins: [Pin] = Fixtures.personas.map(Pin.persona),
        all: [Persona] = Fixtures.personas,
        chats: [ListedChat],
        selected: String?,
        actions: HomeActions,
        @ViewBuilder column: () -> Column,
        @ViewBuilder detail: () -> Detail
    ) {
        self.screen = screen
        self.pins = pins
        self.all = all
        self.chats = chats
        self.selected = selected
        self.actions = actions
        self.column = column()
        self.detail = detail()
    }
}

extension Sidebar where Column == EmptyView, Detail == ChatView {
    public init() {
        self.init(chats: Fixtures.chats, selected: "t1", actions: HomeActions()) {
            EmptyView()
        } detail: {
            ChatView.streaming(back: false)
        }
    }
}
