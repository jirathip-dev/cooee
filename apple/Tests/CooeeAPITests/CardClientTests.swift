@testable import CooeeAPI
import CooeeCore
import Testing

internal struct CardClientTests {
    @Test internal func aCardComesAsItsBlockAndItsAnswerPostsTheItemsChoice() async throws {
        let server = CannedServer([
            "/threads/a1b2/messages": [try Fixture.answer("messages.json")],
            "/threads/a1b2/approve": [Answer(headers: json, chunks: ["null"])],
        ])
        let client = server.client()
        let card = try await client.messages("a1b2").items[3]
        #expect(card.block?.list("blocks").first?.list("options").first?.string("value") == "allow")
        try await client.answer("a1b2", item: 4, option: "save", value: "s3cret")
        #expect(
            try server.decoded("/threads/a1b2/approve", as: [String: JSONValue].self)
                == ["item": .number(4), "option": .string("save"), "value": .string("s3cret")]
        )
    }
}
