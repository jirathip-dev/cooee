import CooeeCore
import Foundation
import Testing

@testable import CooeeAPI

@Suite(.enabled(if: ProcessInfo.processInfo.environment["COOEE_PAIR_LINK"] != nil))
internal struct LiveTests {
    @Test internal func pairsListsAndStreamsAReply() async throws {
        let target = try #require(PairTarget(typed: ProcessInfo.processInfo.environment["COOEE_PAIR_LINK"] ?? ""))
        let token = try await Client(base: target.url).pair(code: try #require(target.code))
        let client = Client(Server(url: target.url, token: token))
        let coder = try #require(try await client.personas().first { $0.id == "coder" })
        let id = try await client.newThread(persona: coder, text: "slow 3")
        #expect(try await client.threads().contains { $0.id == id && $0.persona == "coder" })
        let slice = try await client.messages(id)
        var live = ""
        var turn = slice.turn
        for try await event in client.events(id, since: slice.cursor) {
            if case .text(let text) = event { live += text }
            if case .state(let state) = event { turn = state }
            if live.contains("w2") && turn.working { break }
        }
        #expect(live == "w0 w1 w2 ")
        try await client.cancel(id)
        var items: [APIItem] = []
        for _ in 0..<100 where !items.contains(where: { $0.kind == "agent" }) {
            items = try await client.messages(id).items
            try await Task.sleep(for: .milliseconds(100))
        }
        #expect(items.map(\.kind) == ["you", "agent"])
        #expect(items.last?.text == "streamed 3 words")
    }
}
