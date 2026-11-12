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
            + settings
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

    private static var settings: [Frame] {
        let pages: [(String, MenuFeature.Page)] = [
            ("sign-ins", .signIns), ("devices", .devices), ("machines", .machines), ("plugins", .plugins),
        ]
        let phones = pages.flatMap { name, page in
            [("dark", Palette.farmDark), ("light", Palette.farmLight)].map {
                phone("\(name)-\($0.0)", $0.1) { settingsPage(page) }
            }
        }
        let macs = pages.map { name, page in mac("mac-\(name)", 1180, 780, .farmDark) { SettingsWindow(page: page) } }
        let more = [
            phone("terminal-dark", .farmDark) { MachineTerminal() },
            phone("you-terminal-dark", .farmDark) { MachineTerminal(own: true) },
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
    private static func settingsPage(_ page: MenuFeature.Page) -> some View {
        switch page {
        case .signIns: SignInsView()
        case .devices: DevicesView()
        case .machines: MachinesView()
        default: PluginsView()
        }
    }
}
