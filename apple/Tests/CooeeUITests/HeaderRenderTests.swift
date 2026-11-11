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
        return try #require(Pixels(try #require(ImageRenderer(content: view).cgImage)))
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
        #expect(try Self.pixels(Self.strip([])) == plain)
    }

    @Test internal func theHeaderDrawsPlusOnlyWhenItCanStartAChat() throws {
        var actions = ChatActions()
        let plain = try Self.pixels(actions)
        actions.new = {}
        #expect(try Self.pixels(actions).differing(from: plain, by: 16) > 0.001)
    }
}
