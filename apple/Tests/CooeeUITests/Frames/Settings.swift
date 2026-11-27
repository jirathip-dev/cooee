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

    private static let machines = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent(
            "docs/design/screens-as-data/fixtures/machines.json")

    @MainActor private static func taggedMachines() -> some View {
        let screen = (try? JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: machines))) ?? .null
        return Color.clear.overlay(alignment: .top) { BlockScreen(screen, source: .core).padding(BlockScreen.inset) }
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

    @MainActor
    @Test(arguments: [
        "shell-modern-dark", "shell-modern-light", "shell-scene-light", "shell-terminal-dark", "shell-terminal-light",
        "machine-terminal-dark",
    ])
    internal func theKeyRowIsTheDrawnOne(_ name: String) throws {
        let palette: Palette = name.hasSuffix("dark") ? .farmDark : .farmLight
        let style: Style = name.contains("terminal") ? .terminal : name.contains("scene") ? .scene : .bundled
        let row = TerminalKeyRow(ctrl: name.hasPrefix("machine")).background(palette.background)
            .frame(width: Frame.phoneWidth, height: 46)
            .environment(\.look, Look(palette: palette, style: style, still: true))
            .environment(\.colorScheme, palette.dark ? .dark : .light)
        let renderer = ImageRenderer(content: row)
        renderer.scale = 2
        let top: CGFloat = name.contains("terminal") ? 1098 : 1078
        let strip = CGRect(x: 0, y: top, width: Frame.phoneWidth * 2, height: 92)
        try Frame.matches(try #require(renderer.cgImage), name, in: "composer-v2", cropped: strip)
    }
}
