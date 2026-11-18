import AppKit
import CooeeCore
import CooeeUI
import Foundation
import SwiftUI
import Testing

@MainActor
internal struct HeaderRenderTests {
    private static func pixels(_ actions: ChatActions) throws -> Pixels {
        let persona = Persona(APIPersona(id: "cond", name: "Conductor", agent: "codex"))
        let chat = ChatView(
            persona: persona, items: [], working: false, since: .distantPast, queued: [], back: true, actions: actions
        )
        let view = chat.frame(width: 402, height: 200).environment(\.look, Look(palette: .farmLight, still: true))
        let image = try #require(ImageRenderer(content: view).cgImage)
        return try #require(Pixels(image))
    }

    private static func strip(_ counts: [Int?]) -> ChatActions {
        var actions = ChatActions()
        actions.places = ["Queue", "Events"]
        actions.counts = counts
        return actions
    }

    @Test internal func aPlacesCountDrawsOnItsChip() throws {
        let plain = try Self.pixels(Self.strip([nil, nil]))
        #expect(try Self.pixels(Self.strip([7, nil])).differing(from: plain, by: 16) > 0.001)
        #expect(try Self.pixels(Self.strip([])).differing(from: plain, by: 16) < 0.0001)
    }

    @Test internal func theHeaderDrawsPlusOnlyWhenItCanStartAChat() throws {
        var actions = ChatActions()
        let plain = try Self.pixels(actions)
        actions.new = {}
        #expect(try Self.pixels(actions).differing(from: plain, by: 16) > 0.001)
    }

    private static func gone(_ restart: (() -> Void)?, after: [Item] = []) throws -> Pixels {
        var actions = ChatActions()
        actions.restart = restart
        let items: [Item] = [
            .you(id: 1, text: "Check the build"), .agent(id: 2, markdown: [.text("The build is green.")]),
            .note(id: 3, text: "The agent is gone (shed restarted)."),
        ]
        let persona = Persona(APIPersona(id: "cond", name: "Conductor", agent: "codex"))
        let chat = ChatView(
            persona: persona, items: items + after, working: false, since: .distantPast, queued: [], back: true,
            actions: actions)
        let view = chat.frame(width: 390, height: 520).environment(\.look, Look(palette: .farmLight, still: true))
        let image = try #require(ImageRenderer(content: view).cgImage)
        let name = restart == nil ? "chat-restarted-390" : "chat-gone-restart-390"
        if let folder = ProcessInfo.processInfo.environment["COOEE_RENDERED"] {
            let png = NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:])
            try png?.write(to: URL(fileURLWithPath: folder).appendingPathComponent("\(name).png"))
        }
        return try #require(Pixels(image))
    }

    @Test internal func theGoneLineDrawsRestartOnlyWhileTheAgentIsGone() throws {
        let plain = try Self.gone(nil)
        #expect(try Self.gone {}.differing(from: plain, by: 16) > 0.0005)
        let fresh = Item.note(id: 4, text: "The agent started again fresh: it couldn't pick up where it left off.")
        #expect(try Self.gone(nil, after: [fresh]).differing(from: plain, by: 16) > 0.0005)
    }

    private static func home(loading: Bool) throws -> Pixels {
        let view = HomeView(loading: loading).frame(width: 402, height: 200)
            .environment(\.look, Look(palette: .farmLight, still: true))
        let image = try #require(ImageRenderer(content: view).cgImage)
        return try #require(Pixels(image))
    }

    @Test internal func theHomeLoadingLineSitsOnTheHeaderBottomEdge() throws {
        let idle = try Self.home(loading: false)
        let loading = try Self.home(loading: true)
        let changed = (0..<idle.height).filter { row in
            let start = row * idle.width * 4
            return (start..<start + idle.width * 4).contains { abs(Int(loading.bytes[$0]) - Int(idle.bytes[$0])) > 16 }
        }
        #expect(changed.first == 54 && changed.contains(55) && (changed.last ?? 0) >= 70, "\(changed)")
    }
}
