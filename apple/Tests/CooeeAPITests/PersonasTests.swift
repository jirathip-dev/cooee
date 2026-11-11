@testable import CooeeAPI
import CooeeCore
import Foundation
import Testing

extension APIPersona {
    internal static var coder: APIPersona {
        var coder = APIPersona(
            id: "coder", name: "Coder", agent: "fake-acp", model: "gpt-5.5", effort: "high", folder: "/w",
            character: "Terse.", instructions: "Test first.", plugins: ["hello"]
        )
        coder.avatar = "data:image/png;base64,AA=="
        coder.machine = "air"
        coder.role = "orchestrator"
        coder.fleets = ["cooee"]
        coder.canMessage = ["chief"]
        coder.pinned = true
        coder.profile = "coder"
        return coder
    }
}

internal struct PersonasTests {
    private let persona = try? Fixture.text("personas.json").dropFirst().dropLast(2)

    @Test internal func anEditPutsEveryFieldBackSoNothingTheAppDoesNotDrawIsLost() async throws {
        let one = Answer(headers: json, chunks: [String(try #require(persona))])
        let server = CannedServer(["/personas/coder": [one], "/personas": [one]])
        var edited = APIPersona.coder
        edited.character = "Terse, kind."
        let saved = try await server.client().save(edited)
        #expect(saved == .coder)
        let put = try #require(server.requests("/personas/coder").first)
        #expect(put.httpMethod == "PUT")
        #expect(try server.decoded("/personas/coder", as: APIPersona.self) == edited)
        let sent = try #require(server.body("/personas/coder"))
        #expect(
            sent.contains(#""can_message":["chief"]"#) && sent.contains(#""machine":"air""#)
                && sent.contains(#""pinned":true"#)
                && sent.contains(#""role":"orchestrator""#) && sent.contains(#""fleets":["cooee"]"#))
        _ = try await server.client().create(APIPersona(id: "", name: "Coder", agent: "fake-acp"))
        #expect(server.requests("/personas").first?.httpMethod == "POST")
    }

    @Test internal func memoryReadsAndClears() async throws {
        let notes = #"[{"text":"Renewals live in renewals.md","by":"owner","at":5}]"#
        let server = CannedServer([
            "/personas/coder/memory": [Answer(headers: json, chunks: [notes]), Answer(headers: json, chunks: ["null"])]
        ])
        let client = server.client()
        #expect(
            try await client.memory("coder") == [
                PersonaNote(text: "Renewals live in renewals.md", author: "owner", time: 5)
            ])
        try await client.forget("coder")
        #expect(server.requests("/personas/coder/memory").map(\.httpMethod) == ["GET", "DELETE"])
    }

    @Test internal func pushOnPostsTheDeviceTokenAndOffDeletesIt() async throws {
        let server = CannedServer(["/push/apns": [Answer(chunks: ["ok"])]])
        try await server.client().push("ab12", enabled: true)
        try await server.client().push("ab12", enabled: false)
        #expect(server.requests("/push/apns").map(\.httpMethod) == ["POST", "DELETE"])
        #expect(
            server.requests("/push/apns").map { $0.httpBody }
                == Array(repeating: Data(#"{"token":"ab12"}"#.utf8), count: 2))
    }

    @Test internal func aRefusedSaveCarriesTheServersReason() async throws {
        let refused = Answer(status: 400, headers: ["Cooee-Error": "1"], chunks: ["a persona needs a name"])
        let server = CannedServer(["/personas": [refused]])
        await #expect(throws: APIError(status: 400, text: "a persona needs a name")) {
            try await server.client().create(APIPersona(id: "", name: "", agent: ""))
        }
    }
}
