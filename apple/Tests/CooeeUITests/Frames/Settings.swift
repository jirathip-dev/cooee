import CooeeCore
@testable import CooeeUI
import Foundation
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
        let tagged = [("dark", Palette.farmDark), ("light", Palette.farmLight)].map {
            Frame.phone("machines-tags-\($0.0)", $0.1) { taggedMachines() }
        }
        return (phones + macs + more + tagged).map { $0.filed("v2-settings") }
    }

    internal static let shells: [Frame] = {
        let looks =
            [("modern-dark", Palette.farmDark), ("modern-light", .farmLight), ("scene-light", .farmLight)]
            + [("terminal-dark", Palette.farmDark), ("terminal-light", .farmLight)]
        let shells = looks.map { look, palette in Frame.phone("shell-\(look)", palette) { TerminalView.own() } }
        let machine = Frame.phone("machine-terminal-dark", .farmDark) {
            var view = TerminalView.agents()
            view.ctrl = true
            return view
        }
        return (shells + [machine]).map { $0.filed("composer-v2/frames") }
    }()

    private static let machines = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent(
            "docs/design/screens-as-data/fixtures/machines.json")

    @MainActor private static func taggedMachines() -> some View {
        let screen = (try? JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: machines))) ?? .null
        return Color.clear.overlay(alignment: .top) { BlocksView(screen, source: .core).padding(BlocksView.inset) }
            .clipped()
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

    @MainActor @Test(arguments: shells)
    internal func theKeyRowMatchesTheDesign(_ frame: Frame) throws {
        let style: Style =
            frame.name.contains("terminal") ? .terminal : frame.name.contains("scene") ? .scene : .bundled
        try frame.matchesItsDesign(style)
    }
}
