import CooeeCore
import CooeeFeatures
import CooeeUI
import SwiftUI
import Testing

internal struct Frame: Sendable, CustomTestStringConvertible {
    internal let name: String
    internal let width: CGFloat
    internal let height: CGFloat
    internal let palette: Palette
    internal let style: LookStyle
    internal let phone: Bool
    internal var folder = "v1"
    internal let view: @MainActor @Sendable () -> AnyView

    internal var testDescription: String { name }

    internal func styled(_ other: LookStyle) -> Frame {
        Frame(
            name: "\(name)@\(other.rawValue)", width: width, height: height, palette: palette, style: other,
            phone: phone, folder: folder, view: view
        )
    }

    @MainActor
    internal func render(margin: CGFloat = 0, around: Color = .clear) -> CGImage? {
        let look = Look(palette: palette, style: .bundled(style), still: true)
        let content = Group {
            if phone { PhoneChrome { view() } } else { MacChrome { view() } }
        }
        .frame(width: width, height: height).background(palette.background)
        .environment(\.look, look).environment(\.colorScheme, palette.dark ? .dark : .light)
        .padding(margin).background(around)
        let renderer = ImageRenderer(content: content)
        renderer.scale = 2
        return renderer.cgImage
    }
}

extension Frame {
    private static let phoneWidth: CGFloat = 402
    private static let phoneHeight: CGFloat = 874

    internal static var all: [Frame] {
        designs + styled + settings + missingPoster + additions + QueueFrames.all + DiagramFrames.all
            + ThemeFrames.all
    }

    internal static var everyStyle: [Frame] {
        designs.flatMap { frame in LookStyle.allCases.map(frame.styled) }
    }

    private static var designs: [Frame] {
        [("dark", Palette.farmDark), ("light", Palette.farmLight)].flatMap { phoneFrames($0.0, $0.1) } + mac + rooms
    }

    private static func phone(
        _ name: String,
        _ palette: Palette,
        style: LookStyle = .modern,
        _ view: @escaping @MainActor @Sendable () -> some View
    ) -> Frame {
        Frame(
            name: name, width: phoneWidth, height: phoneHeight, palette: palette, style: style, phone: true,
            view: { AnyView(view()) }
        )
    }

    private static func mac(
        _ name: String,
        _ width: CGFloat,
        _ height: CGFloat,
        _ palette: Palette,
        _ view: @escaping @MainActor @Sendable () -> some View
    ) -> Frame {
        Frame(
            name: name, width: width, height: height, palette: palette, style: .modern, phone: false,
            view: { AnyView(view()) }
        )
    }

    private static func phoneFrames(_ mode: String, _ palette: Palette) -> [Frame] {
        [
            phone("home-\(mode)", palette) { HomeView() },
            phone("chat-streaming-\(mode)", palette) { ChatView.streaming() },
            phone("chat-card-\(mode)", palette) { ChatView.cards() },
            phone("chat-cards-more-\(mode)", palette) { ChatView.moreCards() },
            phone("chat-conductor-\(mode)", palette) { ChatView.conductor() },
            phone("persona-\(mode)", palette) { PersonaEditBlocks() },
            phone("avatar-menu-\(mode)", palette) { HomeView(menuOpen: true) },
            phone("persona-panel-\(mode)", palette) { PersonaPanelBlocks() },
        ]
            + PhoneStep.allCases.map { step in
                phone("pair-\(step.rawValue)-\(mode)", palette) { PairingBlocks(step: step) }
            }
    }

    private static var styled: [Frame] {
        [
            phone("style-terminal-home-dark", .farmDark, style: .terminal) { HomeView() },
            phone("style-terminal-chat-dark", .farmDark, style: .terminal) { ChatView.streaming() },
            phone("style-terminal-card-dark", .farmDark, style: .terminal) { ChatView.cards() },
            phone("style-terminal-persona-dark", .farmDark, style: .terminal) { PersonaEditBlocks() },
            phone("style-scene-home-light", .farmLight, style: .scene) { HomeView() },
            phone("style-scene-chat-light", .farmLight, style: .scene) { ChatView.streaming() },
            phone("style-scene-card-light", .farmLight, style: .scene) { ChatView.cards() },
            phone("persona-panel-end-dark", .farmDark) { PersonaPanelBlocks(end: true) },
            phone("persona-panel-empty-dark", .farmDark) { PersonaPanelBlocks(personaID: "chief") },
            phone("persona-panel-terminal-dark", .farmDark, style: .terminal) { PersonaPanelBlocks() },
            phone("persona-panel-scene-light", .farmLight, style: .scene) { PersonaPanelBlocks() },
        ]
    }

    private static var mac: [Frame] {
        [
            mac("mac-connect", 960, 640, .farmLight) { MacConnect() },
            mac("mac-window-dark", 1280, 820, .farmDark) { Sidebar() },
            mac("mac-window-light", 1280, 820, .farmLight) { Sidebar() },
            mac("mac-persona-panel-dark", 360, 820, .farmDark) { PersonaPanelBlocks(personaID: "coder", column: true) },
        ]
    }

    private static var settings: [Frame] {
        let pages: [(String, SettingsFrame)] = [
            ("sign-ins", .signIns), ("devices", .devices), ("machines", .machines), ("plugins", .plugins),
        ]
        let phones = pages.flatMap { name, page in
            [("dark", Palette.farmDark), ("light", Palette.farmLight)].map {
                phone("\(name)-\($0.0)", $0.1) { settingsPage(page) }
            }
        }
        let macs = pages.map { name, page in mac("mac-\(name)", 1180, 780, .farmDark) { SettingsWindow(page: page) } }
        let more = [
            phone("terminal-dark", .farmDark) { TerminalView.agents() },
            phone("you-terminal-dark", .farmDark) { TerminalView.own() },
            phone("you-terminal-terminal-dark", .farmDark, style: .terminal) { TerminalView.own() },
            phone("you-terminal-scene-light", .farmLight, style: .scene) { TerminalView.own() },
            phone("sign-ins-terminal-dark", .farmDark, style: .terminal) { SignInsView() },
            phone("machines-scene-light", .farmLight, style: .scene) { MachinesView() },
            mac("mac-plugins-light", 1180, 780, .farmLight) { SettingsWindow(page: .plugins) },
        ]
        return (phones + macs + more).map { frame in
            var frame = frame
            frame.folder = "v2-settings"
            return frame
        }
    }

    @MainActor @ViewBuilder
    private static func settingsPage(_ page: SettingsFrame) -> some View {
        switch page {
        case .signIns: SignInsView()
        case .devices: DevicesView()
        case .machines: MachinesView()
        default: PluginsView()
        }
    }

    private static var missingPoster: [Frame] {
        CardTrust.guarded.sorted().flatMap { kind in
            let card = JSONValue(json: #"{"type": "card", "id": "c", "kind": "\#(kind)", "title": "EDGAR_API_KEY"}"#)
            let frame = phone("missing-poster-\(kind)", .farmDark) {
                Color.clear.overlay(alignment: .top) { BlocksView(card, source: nil).padding(BlocksView.inset) }
            }
            return LookStyle.allCases.map(frame.styled)
        }
    }

    private static var additions: [Frame] {
        BlockAdditions.Addition.allCases.flatMap { addition in
            let wide = addition == .refresh
            let frame = Frame(
                name: "additions-\(addition.rawValue)", width: wide ? 960 : phoneWidth,
                height: wide ? 640 : phoneHeight,
                palette: .farmDark, style: .modern, phone: !wide, folder: "v3-additions",
                view: { AnyView(BlockAdditions(addition)) }
            )
            return [frame, frame.styled(.terminal), frame.styled(.scene)]
        }
    }

    private static var rooms: [Frame] {
        [
            phone("room-dark", .farmDark) { RoomView.kestrel() },
            phone("room-light", .farmLight) { RoomView.kestrel() },
            phone("room-create-dark", .farmDark) { NewRoomSheet.kestrel() },
            phone("room-create-light", .farmLight) { NewRoomSheet.kestrel() },
            phone("room-member-dark", .farmDark) { MemberSheet.researcher() },
            phone("room-home-dark", .farmDark) { HomeView(chats: Fixtures.roomChats) },
            phone("room-pinned-dark", .farmDark) { HomeView(pins: Fixtures.roomPins, chats: Fixtures.roomChats) },
            phone("room-pinned-terminal-dark", .farmDark, style: .terminal) {
                HomeView(pins: Fixtures.roomPins, chats: Fixtures.roomChats)
            },
            phone("room-pinned-scene-light", .farmLight, style: .scene) {
                HomeView(pins: Fixtures.roomPins, chats: Fixtures.roomChats)
            },
            phone("room-terminal-dark", .farmDark, style: .terminal) { RoomView.kestrel() },
            phone("room-mention-dark", .farmDark) { RoomView.mentioning() },
            phone("room-mention-terminal-dark", .farmDark, style: .terminal) { RoomView.mentioning() },
            phone("room-scene-light", .farmLight, style: .scene) { RoomView.kestrel() },
            phone("room-ask-dark", .farmDark) { RoomView.asking() },
            phone("room-ask-terminal-dark", .farmDark, style: .terminal) { RoomView.asking() },
            phone("room-ask-scene-light", .farmLight, style: .scene) { RoomView.asking() },
            mac("room-wide-dark", 1440, 900, .farmDark) { Sidebar.kestrel() },
            phone("asked-dark", .farmDark) { ChatView.asked() },
            phone("asked-light", .farmLight) { ChatView.asked() },
            phone("linked-dark", .farmDark) { LinkedView.researcher() },
            phone("linked-terminal-dark", .farmDark, style: .terminal) { LinkedView.researcher() },
            mac("linked-wide-light", 1440, 900, .farmLight) { Sidebar.linked() },
            Frame(
                name: "room-terminal-wide", width: 1440, height: 900, palette: .farmDark, style: .terminal,
                phone: false,
                view: { AnyView(Sidebar.kestrel()) }
            ),
        ]
    }
}
