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
}
