import CooeeCore
import CooeeUI
import SwiftUI
import Testing

@Suite(.serialized)
internal struct SettingsFrames {
    internal static var all: [Frame] {
        let pages: [(String, SettingsFrame)] = [
            ("sign-ins", .signIns), ("devices", .devices), ("machines", .machines), ("plugins", .plugins),
        ]
        let phones = pages.flatMap { name, page in
            [("dark", Palette.farmDark), ("light", Palette.farmLight)].map {
                Frame.phone("\(name)-\($0.0)", $0.1) { settingsPage(page) }
            }
        }
        let macs = pages.map { name, page in
            Frame.mac("mac-\(name)", 1180, 780, .farmDark) { SettingsWindow(page: page) }
        }
        let more = [
            Frame.phone("terminal-dark", .farmDark) { TerminalView.agents() },
            .phone("you-terminal-dark", .farmDark) { TerminalView.own() },
            .mac("mac-plugins-light", 1180, 780, .farmLight) { SettingsWindow(page: .plugins) },
        ]
        return (phones + macs + more).map { $0.filed("v2-settings") }
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

    @MainActor @Test(arguments: all)
    internal func matchesTheDesign(_ frame: Frame) throws { try frame.matchesItsDesign() }
}
