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

    private static func thread(window: CGFloat) throws -> CGImage {
        let persona = Persona(
            APIPersona(id: "cond", name: "Conductor", agent: "fake-acp-agent", model: "frontier-large-2026-11-05"))
        var actions = ChatActions()
        actions.machine = MachineRow(name: "studio-mac-mini.tailnet", host: "203.0.113.7")
        actions.screen = {}
        let row = ThreadRow(
            id: "t1", title: "release notes for the November build", agent: "fake-acp-agent", persona: "cond",
            state: "done", last: "", updated: nil, unseen: 0, archived: false)
        actions.topics = TopicActions(topics: Topics([row]), current: "t1")
        let failed = Outgoing(
            id: 1, text: "Tag the release once the notes are merged and the build is green",
            failure: "the agent stopped, so it can't take the message")
        let chat = ChatView(
            persona: persona, items: [.outgoing(failed)], working: false, since: .distantPast, queued: [],
            back: window < 700, actions: actions
        )
        let renderer = ImageRenderer(content: chat.environment(\.look, Look(palette: .farmLight, still: true)))
        renderer.proposedSize = ProposedViewSize(width: window, height: 844)
        renderer.scale = 1
        return try #require(renderer.cgImage)
    }

    @Test(arguments: [390, 1440] as [CGFloat])
    internal func aThreadIsNoWiderThanItsWindow(_ window: CGFloat) throws {
        #expect(try Self.thread(window: window).width <= Int(window))
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
