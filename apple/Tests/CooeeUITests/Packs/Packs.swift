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
    private static var terminal: [Frame] {
        [
            .phone("style-terminal-home-dark", .farmDark) { HomeView() },
            .phone("style-terminal-chat-dark", .farmDark) { ChatView.streaming() },
            .phone("style-terminal-card-dark", .farmDark) { ChatView.cards() },
            .phone("style-terminal-persona-dark", .farmDark) { PersonaEditBlocks() },
            .phone("persona-panel-terminal-dark", .farmDark) { PersonaPanelBlocks() },
            .phone("you-terminal-terminal-dark", .farmDark) { TerminalView.own() },
            .phone("sign-ins-terminal-dark", .farmDark) { SignInsView() },
            .phone("room-pinned-terminal-dark", .farmDark) {
                HomeView(pins: Fixtures.roomPins, chats: Fixtures.roomChats)
            },
            .phone("room-terminal-dark", .farmDark) { RoomView.kestrel() },
            .phone("room-mention-terminal-dark", .farmDark) { RoomView.mentioning() },
            .phone("room-ask-terminal-dark", .farmDark) { RoomView.asking() },
            .phone("linked-terminal-dark", .farmDark) { LinkedView.researcher() },
            .mac("room-terminal-wide", 1440, 900, .farmDark) { Sidebar.kestrel() },
            DiagramFrames.frame("map-terminal-dark") { DiagramFrames.sheet(map) },
            DiagramFrames.frame("map-mac-terminal-dark", phone: false) { DiagramFrames.sheet(map, width: 760) },
            TopicFrames.frame("press", "terminal", .farmLight) { TopicFrames.chat(pressed: "c2") },
        ] + shared("terminal", .farmDark) + ThemeFrames.editor("terminal")
    }

    private static var scene: [Frame] {
        [
            .phone("style-scene-home-light", .farmLight) { HomeView() },
            .phone("style-scene-chat-light", .farmLight) { ChatView.streaming() },
            .phone("style-scene-card-light", .farmLight) { ChatView.cards() },
            .phone("persona-panel-scene-light", .farmLight) { PersonaPanelBlocks() },
            .phone("you-terminal-scene-light", .farmLight) { TerminalView.own() },
            .phone("machines-scene-light", .farmLight) { MachinesView() },
            .phone("room-pinned-scene-light", .farmLight) {
                HomeView(pins: Fixtures.roomPins, chats: Fixtures.roomChats)
            },
            .phone("room-scene-light", .farmLight) { RoomView.kestrel() },
            .phone("room-ask-scene-light", .farmLight) { RoomView.asking() },
            DiagramFrames.frame("map-scene-light", .farmLight) { DiagramFrames.sheet(map) },
        ] + shared("scene", .farmLight)
    }

    private static var map: JSONValue { DiagramFrames.fixture("workflow-diagram/fixtures/map.json") }

    private static func shared(_ pack: String, _ palette: Palette) -> [Frame] {
        TrustFrames.posters(pack) + AdditionFrames.additions("@\(pack)") + QueueFrames.sheets(pack)
            + TopicFrames.topics(pack, palette)
    }

    internal static let all =
        terminal.map { PackFrame(pack: "terminal", frame: $0) } + scene.map { PackFrame(pack: "scene", frame: $0) }

    @MainActor @Test(arguments: all)
    internal func matchesThePack(_ frame: PackFrame) throws { try frame.matchesItsDesign() }
}
