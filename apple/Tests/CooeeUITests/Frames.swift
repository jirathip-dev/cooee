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
    internal let view: @MainActor @Sendable () -> AnyView

    internal var testDescription: String { name }

    @MainActor
    internal func render() -> CGImage? {
        let look = Look(palette: palette, style: style, still: true)
        let content = Group {
            if phone { PhoneChrome { view() } } else { MacChrome { view() } }
        }
        .frame(width: width, height: height).background(palette.background)
        .environment(\.look, look).environment(\.colorScheme, palette.dark ? .dark : .light)
        let renderer = ImageRenderer(content: content)
        renderer.scale = 2
        return renderer.cgImage
    }
}

extension Frame {
    private static let phoneWidth: CGFloat = 402
    private static let phoneHeight: CGFloat = 874

    internal static var all: [Frame] {
        [("dark", Palette.farmDark), ("light", Palette.farmLight)].flatMap { phoneFrames($0.0, $0.1) } + styled + mac
            + rooms
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
            phone("persona-\(mode)", palette) { PersonaSheet() },
            phone("avatar-menu-\(mode)", palette) { HomeView(menuOpen: true) },
        ]
            + PhoneStep.allCases.map { step in
                phone("pair-\(step.rawValue)-\(mode)", palette) { PhoneOnboarding(step: step) }
            }
    }

    private static var styled: [Frame] {
        [
            phone("home-terminal-dark", .farmDark, style: .terminal) { HomeView() },
            phone("chat-terminal-dark", .farmDark, style: .terminal) { ChatView.streaming() },
            phone("home-scene-light", .farmLight, style: .scene) { HomeView() },
        ]
    }

    private static var mac: [Frame] {
        MacStep.allCases.map { step in mac("mac-setup-\(step.rawValue)", 960, 640, .farmDark) { MacSetup(step: step) } }
            + [
                mac("mac-connect", 960, 640, .farmLight) { MacConnect() },
                mac("mac-window-dark", 1280, 820, .farmDark) { WideView() },
                mac("mac-window-light", 1280, 820, .farmLight) { WideView() },
            ]
    }

    private static var rooms: [Frame] {
        [
            phone("room-dark", .farmDark) { RoomView.kestrel() },
            phone("room-light", .farmLight) { RoomView.kestrel() },
            phone("room-create-dark", .farmDark) { NewRoomSheet.kestrel() },
            phone("room-create-light", .farmLight) { NewRoomSheet.kestrel() },
            phone("room-member-dark", .farmDark) { MemberSheet.researcher() },
            phone("room-home-dark", .farmDark) { HomeView(chats: Fixtures.roomChats) },
            phone("room-terminal-dark", .farmDark, style: .terminal) { RoomView.kestrel() },
            phone("room-scene-light", .farmLight, style: .scene) { RoomView.kestrel() },
            mac("room-wide-dark", 1440, 900, .farmDark) { WideView.kestrel() },
            Frame(
                name: "room-terminal-wide", width: 1440, height: 900, palette: .farmDark, style: .terminal,
                phone: false,
                view: { AnyView(WideView.kestrel()) }
            ),
        ]
    }
}
