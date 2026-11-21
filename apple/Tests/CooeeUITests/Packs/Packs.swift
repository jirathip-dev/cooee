import CooeeCore
@testable import CooeeUI
import Foundation
import SwiftUI
import Testing

extension Style {
    private static let folder = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .appendingPathComponent("packs")

    private static func json(_ path: String) -> Data {
        (try? Data(contentsOf: folder.appendingPathComponent(path))) ?? Data()
    }

    private static func read(_ path: String) -> Style {
        // SAFETY: DefaultLayoutTests decodes both packs and the worst-case fixture, so a bad edit fails there by name
        try! JSONDecoder().decode(Style.self, from: json(path))
    }

    internal static let terminal = read("terminal/style.json")
    internal static let scene = read("scene/style.json")
    internal static let worstCase = read("fixtures/styles/ok-worst-case.json")
    internal static let packs = [terminal, scene]

    internal static let served: [String: ServedStyle] = ["terminal", "scene"].reduce(into: [:]) { styles, name in
        styles[name] = try? JSONDecoder().decode(ServedStyle.self, from: json("\(name)/style.json"))
    }
}

internal struct PackFrame: Sendable, CustomTestStringConvertible {
    internal let pack: String
    internal let frame: Frame

    internal var testDescription: String { frame.name }

    @MainActor
    internal func matchesItsDesign() throws {
        try frame.matchesItsDesign(pack == "terminal" ? .terminal : .scene, in: "packs/\(pack)")
    }
}

@Suite(.serialized)
internal struct PackFrames {
    private static func sheet(_ pack: String, _ mode: String, _ palette: Palette) -> [Frame] {
        [
            .phone("style-\(pack)-home-\(mode)", palette) { HomeView() },
            .phone("style-\(pack)-chat-\(mode)", palette) { ChatView.streaming() },
            .phone("style-\(pack)-card-\(mode)", palette) { ChatView.cards() },
            .phone("persona-panel-\(pack)-\(mode)", palette) { PersonaPanelBlocks() },
        ] + TrustFrames.posters(pack)
    }

    private static let terminal = sheet("terminal", "dark", .farmDark)
    private static let scene = sheet("scene", "light", .farmLight) + skies

    private static let skies: [Frame] = [
        ("clear", 0.2, false), ("cloudy", 0.6, false), ("storm", 1.2, false),
        ("night", 0.2, true),
    ].map { name, load, night in
        .phone("scene-sky-\(name)-light", .farmLight) { SkyStrip(load: load, night: night) }
    }

    internal static let all =
        terminal.map { PackFrame(pack: "terminal", frame: $0) } + scene.map { PackFrame(pack: "scene", frame: $0) }

    @MainActor @Test(arguments: all)
    internal func matchesThePack(_ frame: PackFrame) throws { try frame.matchesItsDesign() }
}

private struct SkyStrip: View {
    fileprivate let load: Double
    fileprivate let night: Bool

    fileprivate var body: some View {
        let lanes = ["needs", "working", "done", "idle"].enumerated().map { index, state in
            JSONValue.object(["id": .string("l\(index)"), "title": .string("#\(index + 1)"), "state": .string(state)])
        }
        let scene = JSONValue.object([
            "type": .string("scene"), "label": .string("lanes"), "lanes": .array(lanes), "load": .number(load),
            "next": .object(["action": .string("filter"), "values": .object(["repo": .string("cooee")])]),
        ])
        Color.clear.overlay(alignment: .top) { BlocksView(scene, source: .plugin("drover")).padding(BlocksView.inset) }
            .transformEnvironment(\.look) { $0.night = night }
    }
}
