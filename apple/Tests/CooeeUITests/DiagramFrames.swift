import CooeeCore
import CooeeUI
import Foundation
import SwiftUI

internal enum DiagramFrames {
    private static let docs = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("docs/design")

    private static func fixture(_ path: String) -> JSONValue {
        let data = (try? Data(contentsOf: docs.appendingPathComponent(path))) ?? Data()
        return (try? JSONDecoder().decode(JSONValue.self, from: data)) ?? .null
    }

    private static var live: JSONValue {
        var screen = LiveScreen(fixture("workflow-diagram/fixtures/map.json"))
        for event in fixture("workflow-diagram/fixtures/stream.json").items { screen.apply(event: event) }
        return screen.root
    }

    @MainActor private static func sheet(
        _ screen: JSONValue, focus: String? = nil, width: CGFloat? = nil, beside: (String, JSONValue)? = nil
    ) -> some View {
        Color.clear.overlay(alignment: .top) {
            BlocksView(screen, source: .plugin("drover"), focus: focus, picked: beside?.0, beside: beside?.1).padding(
                BlocksView.inset
            ).frame(
                width: width
            ).padding(
                .top, width == nil ? 0 : 36)
        }
        .clipped()
    }

    private static func frame(
        _ name: String, _ palette: Palette = .farmDark, style: LookStyle = .modern, phone: Bool = true,
        _ view: @escaping @MainActor @Sendable () -> some View
    ) -> Frame {
        Frame(
            name: "workflow-\(name)", width: phone ? 402 : 1440, height: phone ? 874 : 900, palette: palette,
            style: style, phone: phone, view: { AnyView(view()) })
    }

    internal static var all: [Frame] {
        let map = fixture("workflow-diagram/fixtures/map.json")
        let fix = fixture("workflow-diagram/fixtures/fix.json")
        let global = fixture("fleets-config/fixtures/global.json")
        return [
            frame("map-dark") { sheet(map) }, frame("map-light", .farmLight) { sheet(map) },
            frame("map-terminal-dark", style: .terminal) { sheet(map) },
            frame("map-scene-light", .farmLight, style: .scene) { sheet(map) },
            frame("map-mac-dark", phone: false) { sheet(map, width: 760) },
            frame("map-mac-split-dark", phone: false) {
                sheet(map, width: 1090, beside: ("/screens/queue?state=fix", fix))
            },
            frame("map-mac-terminal-dark", style: .terminal, phone: false) { sheet(map, width: 760) },
            frame("live-dark") { sheet(live) }, frame("state-dark") { sheet(fix) },
            frame("timer-dark") { sheet(global, focus: "limits.stall_minutes") },
        ]
    }
}
