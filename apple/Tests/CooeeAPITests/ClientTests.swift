@testable import CooeeAPI
import CooeeCore
import Foundation
import Testing

internal struct ClientTests {
    private struct Sent: Decodable, Equatable {
        private enum CodingKeys: String, CodingKey {
            case text, files
            case randomID = "random_id"
        }

        fileprivate let text: String
        fileprivate let randomID: String
        fileprivate let files: [CooeeCore.Attachment]?
    }

    private struct Started: Decodable, Equatable {
        private enum CodingKeys: String, CodingKey {
            case agent, profile, text, background
            case randomID = "random_id"
        }

        fileprivate let agent: String
        fileprivate let profile: String
        fileprivate let text: String
        fileprivate let randomID: String
        fileprivate let background: Bool
    }

    @Test internal func threadsPersonasAndASliceDecodeAsDocumented() async throws {
        let server = CannedServer([
            "/threads": [try Fixture.answer("threads.json")],
            "/personas": [try Fixture.answer("personas.json")],
            "/threads/a1b2/messages": [try Fixture.answer("messages.json")],
        ])
        let client = server.client()
        let rows = try await client.threads()
        #expect(rows.map(\.id) == ["a1b2", "old"])
        #expect(rows[0].persona == "coder" && rows[0].unseen == 2 && rows[0].state == "working" && rows[1].archived)
        let personas = try await client.personas()
        #expect(
            personas == [
                APIPersona(
                    id: "coder", name: "Coder", agent: "fake-acp", model: "gpt-5.5", effort: "high", folder: "/w",
                    character: "Terse.", instructions: "Test first.", plugins: ["hello"]
                )
            ]
        )
        let slice = try await client.messages("a1b2")
        #expect(slice.cursor == "boot.5" && slice.turn == Turn(text: "Hel", state: "working", note: "retrying"))
        #expect(slice.queue == [Queued(text: "Then open the PR", randomID: "q1")])
        #expect(
            slice.items.map(\.kind) == ["you", "tool", "tool", "approval", "link", "plan", "note", "agent", "ended"])
        #expect(server.requests("/threads").first?.value(forHTTPHeaderField: "Cookie") == "cooee=tok")
        #expect(server.requests("/threads/a1b2/messages").first?.url?.query == "before=50")
    }

    @Test internal func sendQueueAndTheTurnsCallsPostTheirBodies() async throws {
        let empty = Answer(headers: json, chunks: ["null"])
        let server = CannedServer([
            "/threads/a1b2/send": [empty], "/threads/a1b2/queue": [empty], "/threads/a1b2/steer": [empty],
            "/threads/a1b2/unqueue": [empty], "/threads/a1b2/cancel": [empty], "/threads/a1b2/read": [empty],
        ])
        let client = server.client()
        let file = CooeeCore.Attachment(name: "a.txt", bytes: Data("hi".utf8))
        try await client.send("a1b2", text: "first", files: [file], randomID: "r1")
        try await client.send("a1b2", text: "second", queue: true, randomID: "r2")
        try await client.steer("a1b2", "q1")
        try await client.unqueue("a1b2", "q2")
        try await client.cancel("a1b2")
        try await client.read("a1b2", upTo: 4)
        let sent = try server.decoded("/threads/a1b2/send", as: Sent.self)
        #expect(sent == Sent(text: "first", randomID: "r1", files: [file]) && file.data == "aGk=")
        #expect(
            try server.decoded("/threads/a1b2/queue", as: Sent.self) == Sent(text: "second", randomID: "r2", files: nil)
        )
        #expect(try server.decoded("/threads/a1b2/steer", as: [String: String].self) == ["random_id": "q1"])
        #expect(try server.decoded("/threads/a1b2/unqueue", as: [String: String].self) == ["random_id": "q2"])
        #expect(server.body("/threads/a1b2/cancel") == "{}")
        #expect(server.body("/threads/a1b2/read") == #"{"item":4}"#)
    }

    @Test internal func aNewThreadStartsForAPersona() async throws {
        let server = CannedServer(["/threads": [Answer(headers: json, chunks: [#"{"id":"n1"}"#])]])
        let persona = APIPersona(id: "coder", name: "Coder", agent: "fake-acp")
        #expect(try await server.client().newThread(persona: persona, text: "hi", randomID: "r9") == "n1")
        let body = try server.decoded("/threads", as: Started.self)
        #expect(body == Started(agent: "fake-acp", profile: "coder", text: "hi", randomID: "r9", background: true))
    }

    @Test internal func cooeesOwnSentenceIsTheError() async throws {
        let server = CannedServer([
            "/threads": [Answer(status: 500, headers: ["Cooee-Error": "1"], chunks: ["The store is locked."])],
            "/personas": [Answer(status: 401, chunks: ["nope"])],
        ])
        let client = server.client()
        await #expect(throws: APIError(status: 500, text: "The store is locked.")) { try await client.threads() }
        await #expect(throws: APIError(status: 401, text: "")) { try await client.personas() }
        #expect(APIError(status: 401, text: "").errorDescription == "The server answered 401.")
        #expect(APIError(status: 500, text: "Locked.").errorDescription == "Locked.")
    }

    @Test internal func aCodePairsAndAClaimWaitsThenPairs() async throws {
        let server = CannedServer([
            "/pair": [
                Answer(
                    status: 303, headers: ["Location": "/?paired", "Set-Cookie": "cooee=abc123; Path=/; HttpOnly"],
                    chunks: [])
            ],
            "/pair/request": [Answer(headers: json, chunks: [#"{"word":"maple","claim":"cl1"}"#])],
            "/pair/claim": [
                Answer(status: 202, chunks: ["waiting"]), Answer(chunks: []),
                Answer(headers: ["Set-Cookie": "cooee=def456; Path=/"], chunks: []),
            ],
            "/pair/cancel": [Answer(chunks: ["cancelled"])],
        ])
        let client = server.client(token: nil)
        #expect(try await client.pair(code: "123 456") == "abc123")
        #expect(server.body("/pair") == "code=123456")
        #expect(server.requests("/pair").first?.value(forHTTPHeaderField: "Cookie") == nil)
        let ask = try await client.askToPair()
        #expect(ask == PairAsk(word: "maple", claim: "cl1"))
        #expect(try await client.claim(ask.claim) == .waiting)
        #expect(try await client.claim(ask.claim) == .waiting)
        #expect(try await client.claim(ask.claim) == .paired("def456"))
        try await client.cancelPair(ask.claim)
        #expect(server.body("/pair/cancel") == #"{"claim":"cl1"}"#)
    }

    @Test(arguments: [
        ("/pair?bad=wrong&left=4", "That code is wrong."), ("/pair?bad=expired", "That code ran out: get a new one."),
        ("/pair?bad=limit&wait=30", "Too many wrong codes: wait a minute."),
    ])
    internal func aRefusedCodeSaysWhy(_ location: String, _ why: String) async throws {
        let server = CannedServer(["/pair": [Answer(status: 303, headers: ["Location": location], chunks: [])]])
        await #expect(throws: APIError(status: 303, text: why)) { try await server.client(token: nil).pair(code: "0") }
    }

    @Test internal func aDeclinedOrGoneClaimSaysWhy() async throws {
        let server = CannedServer([
            "/pair/claim": [
                Answer(status: 403, headers: ["Cooee-Error": "1"], chunks: ["mini said no."]),
                Answer(status: 410, headers: ["Cooee-Error": "1"], chunks: ["It ran out."]),
                Answer(status: 500, chunks: []),
            ]
        ])
        let client = server.client(token: nil)
        #expect(try await client.claim("cl1") == .declined("mini said no."))
        #expect(try await client.claim("cl1") == .gone("It ran out."))
        await #expect(throws: APIError(status: 500, text: "")) { try await client.claim("cl1") }
    }

    @Test internal func aClientForAServerCarriesItsCookie() throws {
        let server = Server(url: try #require(URL(string: "https://mini.local")), token: "t1")
        let client = Client(server)
        #expect(client.base == server.url)
        #expect(try client.request("GET", "/threads").value(forHTTPHeaderField: "Cookie") == "cooee=t1")
        #expect(throws: APIError.self) { try Client.http(URLResponse()) }
    }
}
