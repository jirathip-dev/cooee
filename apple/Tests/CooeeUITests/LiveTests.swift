import Foundation
import Testing
@testable import CooeeUI

/// Against a real `cooee serve` (apple/e2e/serve.sh): `COOEE_PAIR_LINK` is its `cooee pair` link. Skipped without it.
@Suite(.enabled(if: ProcessInfo.processInfo.environment["COOEE_PAIR_LINK"] != nil)) struct LiveTests {
    @Test @MainActor func pairsListsAndStreamsAReply() async throws {
        let (url, code) = try #require(target(ProcessInfo.processInfo.environment["COOEE_PAIR_LINK"]!))
        let token = try await Client(base: url).pair(code: try #require(code))
        let home = HomeModel(Client(Server(url: url, token: token)))
        await home.load()
        let coder = try #require(home.persona("coder"), "\(home.error ?? "no coder")")
        let id = try await home.client.newThread(persona: coder, text: "slow 3")
        await home.load()
        #expect(home.list().contains { $0.0.id == id && $0.1.name == "Coder" })
        let chat = ChatModel(home.client, id: id)
        let run = Task { await chat.open() }
        defer { run.cancel() }
        func until(_ ok: () -> Bool) async throws {
            for _ in 0..<1500 where !ok() { try await Task.sleep(for: .milliseconds(20)) }
        }
        // The fake agent streams `w0 w1 w2 ` a chunk at a time and holds the turn open until it is stopped.
        try await until { chat.live.contains("w2") }
        #expect(chat.live == "w0 w1 w2 " && chat.working)
        guard case .agent(_, let live)? = chatItems(chat.items, live: chat.live).last else { Issue.record("no live reply"); return }
        #expect(live == [.text("w0 w1 w2  ▍")])
        chat.stop()
        try await until { chat.items.contains { $0.kind == "agent" } && !chat.working }
        #expect(chat.items.map(\.kind) == ["you", "agent"])
        #expect(chat.items.last?.text == "streamed 3 words" && chat.live == "")
    }
}
