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

    internal func styled(_ other: LookStyle) -> Frame {
        Frame(
            name: "\(name)@\(other.rawValue)", width: width, height: height, palette: palette, style: other,
            phone: phone, view: view
        )
    }

    @MainActor
    internal func render(margin: CGFloat = 0, around: Color = .clear) -> CGImage? {
        let look = Look(palette: palette, style: style, still: true)
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

    internal static var all: [Frame] { designs + styled }

    internal static var everyStyle: [Frame] {
        designs.flatMap { frame in LookStyle.allCases.map(frame.styled) }
    }

    private static var designs: [Frame] {
        [("dark", Palette.farmDark), ("light", Palette.farmLight)].flatMap { phoneFrames($0.0, $0.1) } + mac
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
            phone("style-terminal-home-dark", .farmDark, style: .terminal) { HomeView() },
            phone("style-terminal-chat-dark", .farmDark, style: .terminal) { ChatView.streaming() },
            phone("style-terminal-card-dark", .farmDark, style: .terminal) { ChatView.cards() },
            phone("style-terminal-persona-dark", .farmDark, style: .terminal) { PersonaSheet() },
            phone("style-scene-home-light", .farmLight, style: .scene) { HomeView() },
            phone("style-scene-chat-light", .farmLight, style: .scene) { ChatView.streaming() },
            phone("style-scene-card-light", .farmLight, style: .scene) { ChatView.cards() },
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
}
