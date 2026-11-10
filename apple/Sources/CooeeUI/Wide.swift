import CooeeCore
import SwiftUI

public struct MacConnect: View {
    @Environment(\.look) private var look

    public var body: some View {
        let palette = look.palette
        ZStack {
            Backdrop()
            VStack(spacing: 18) {
                AppMark(size: 80)
                Text("Connect to a server").ui(26, .bold, look)
                Text("cooee runs on another Mac or a Linux box. This Mac only talks to it.").ui(14, .regular, look)
                    .foregroundStyle(palette.mute)
                HStack(alignment: .top, spacing: 18) {
                    VStack(alignment: .leading, spacing: 10) {
                        Text("NEARBY").ui(11, .semibold, look).foregroundStyle(palette.mute)
                        NearbyList()
                    }
                    address
                }
                .frame(maxWidth: 760)
                Spacer()
                Text("Want this Mac to be the server? Set it up instead").ui(13, .regular, look)
                    .foregroundStyle(palette.link)
            }
            .padding(36)
        }
        .foregroundStyle(palette.foreground)
    }

    private var address: some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 10) {
            Text("ADDRESS").ui(11, .semibold, look).foregroundStyle(palette.mute)
            AddressField()
            HStack(spacing: 8) {
                Image(systemName: "qrcode.viewfinder").foregroundStyle(palette.link)
                Text("Scan its QR with this Mac's camera").ui(14, .regular, look).foregroundStyle(palette.link)
            }
            .padding(.top, 8)
        }
    }

    public init() {}
}

public struct WideView<Detail: View>: View {
    @Environment(\.look) private var look
    private let personas: [Persona]
    private let all: [Persona]
    private let chats: [ListedChat]
    private let selected: String?
    private let actions: HomeActions
    private let detail: Detail

    public var body: some View {
        let palette = look.palette
        HStack(spacing: 0) {
            VStack(spacing: 0) {
                TopBar(personas: all, actions: actions).padding(.top, look.still ? 34 : 8)
                Scroller {
                    PinnedRow(personas: personas, rows: chats.map(\.row), new: actions.new)
                        .scaleEffect(0.9, anchor: .leading).padding(.top, 4)
                    ChatList(chats: chats, selected: selected, open: actions.open).padding(.top, 8)
                }
            }
            .frame(width: 360).background(palette.card.opacity(look.style == .modern ? 0.45 : 0.8))
            .overlay(alignment: .trailing) { Rectangle().fill(palette.line).frame(width: 1) }
            detail.padding(.top, 12).frame(maxWidth: .infinity)
        }
        .foregroundStyle(palette.foreground)
    }

    public init(
        personas: [Persona],
        all: [Persona]? = nil,
        chats: [ListedChat],
        selected: String?,
        actions: HomeActions,
        @ViewBuilder detail: () -> Detail
    ) {
        self.personas = personas
        self.all = all ?? personas
        self.chats = chats
        self.selected = selected
        self.actions = actions
        self.detail = detail()
    }
}

extension WideView where Detail == ChatView {
    public init() {
        self.init(personas: Fixtures.personas, chats: Fixtures.chats, selected: "t1", actions: HomeActions()) {
            ChatView.streaming(back: false)
        }
    }
}

public struct MacChrome<Content: View>: View {
    private let content: Content

    public var body: some View {
        ZStack(alignment: .topLeading) {
            content
            HStack(spacing: 8) {
                Circle().fill(Color.hex(0xff5f57))
                Circle().fill(Color.hex(0xfebc2e))
                Circle().fill(Color.hex(0x28c840))
            }
            .frame(width: 52, height: 12).padding(.leading, 14).padding(.top, 14)
        }
        .clipShape(.rect(cornerRadius: 12))
    }

    public init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }
}
