@testable import CooeeCore
import Foundation
import Testing

internal struct ListingTests {
    private let coder = APIPersona(id: "coder", name: "Coder", agent: "codex")
    private let chief = APIPersona(id: "chief", name: "Chief", agent: "claude")
    private let now = Date(timeIntervalSince1970: 1_000_000)

    private func row(_ id: String, persona: String, state: String, updated seconds: Int64) -> ThreadRow {
        ThreadRow(
            id: id, title: "T \(id)", agent: "fake-acp", persona: persona, state: state, last: "last \(id)",
            updated: seconds * 1000, unseen: 1, archived: false
        )
    }

    @Test internal func eachPinnedPersonaIsInItsNewestChatsState() {
        let rows = [
            row("t1", persona: "coder", state: "working", updated: 999_990),
            row("t2", persona: "coder", state: "done", updated: 900_000),
        ]
        let pinned = Listing.pinned(personas: [coder, chief], rows: rows)
        #expect(pinned.map(\.id) == ["coder", "chief"])
        #expect(pinned.map(\.state) == [.working, .idle])
    }

    @Test internal func theChatListDrawsEachRowWithItsPersonaOrItsAgent() {
        let rows = [
            row("t1", persona: "coder", state: "blocked", updated: 999_990),
            row("t2", persona: "", state: "done", updated: 999_000),
        ]
        let chats = Listing.chats(rows: rows, personas: [coder], now: now)
        #expect(chats.map(\.id) == ["t1", "t2"])
        #expect(
            chats[0].row
                == Row(id: "t1", persona: "coder", title: "T t1", last: "last t1", ago: "now", unseen: 1, state: .needs)
        )
        #expect(chats[0].persona.name == "Coder" && chats[0].persona.state == .needs)
        #expect(chats[1].persona.name == "fake-acp" && chats[1].row.persona == "fake-acp" && chats[1].row.ago == "16m")
    }

    @Test(arguments: [
        (999_970, "now"), (999_000, "16m"), (990_000, "2h"), (800_000, "2d"), (1_000_500, "now"),
    ])
    internal func agoIsShort(_ seconds: Int64, _ text: String) {
        #expect(Listing.ago(seconds * 1000, now: now) == text)
    }

    @Test internal func aTypedAddressOrALinkIsAServer() {
        #expect(PairTarget(typed: "mini.local:4100")?.url.absoluteString == "https://mini.local:4100")
        let link = PairTarget(typed: " http://127.0.0.1:4100/pair?code=123456 ")
        #expect(link?.url.absoluteString == "http://127.0.0.1:4100" && link?.code == "123456")
        #expect(PairTarget(typed: "  ") == nil)
        #expect(PairTarget(typed: "http://") == nil)
    }
}
