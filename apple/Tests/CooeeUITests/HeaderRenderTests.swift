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

    private static func home(loading: Bool) throws -> Pixels {
        let view = HomeView(loading: loading).frame(width: 402, height: 200)
            .environment(\.look, Look(palette: .farmLight, still: true))
        let image = try #require(ImageRenderer(content: view).cgImage)
        return try #require(Pixels(image))
    }

    @Test internal func theHomeLoadingLineSitsOnTheHeaderBottomEdge() throws {
        let idle = try Self.home(loading: false)
        let loading = try Self.home(loading: true)
        let row = idle.width * 4
        #expect(loading.bytes[..<(54 * row)] == idle.bytes[..<(54 * row)])
        #expect(loading.bytes[(54 * row)..<(56 * row)] != idle.bytes[(54 * row)..<(56 * row)])
        #expect(loading.bytes[(56 * row)..<(75 * row)] != idle.bytes[(56 * row)..<(75 * row)])
    }
}
