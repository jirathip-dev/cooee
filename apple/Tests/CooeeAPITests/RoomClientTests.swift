@testable import CooeeAPI
import CooeeCore
import Testing

internal struct RoomClientTests {
    @Test internal func aRoomIsMadeOfItsMembersInRoomOrder() async throws {
        let server = CannedServer([
            "/rooms": [Answer(headers: json, chunks: [#"{"id":"r1"}"#])],
            "/threads": [
                Answer(
                    headers: json,
                    chunks: [
                        #"[{"id":"r1","title":"Kestrel","agent":"","persona":"","state":"","last":"Hi","at":1,"#
                            + #""unseen":0,"archived":false,"members":["chief","am"]}]"#
                    ]
                )
            ],
        ])
        let client = server.client()
        #expect(try await client.newRoom(title: "Kestrel", members: ["chief", "am"], randomID: "n1") == "r1")
        #expect(
            try server.decoded("/rooms", as: [String: JSONValue].self)
                == [
                    "title": .string("Kestrel"), "members": .array([.string("chief"), .string("am")]),
                    "random_id": .string("n1"),
                ]
        )
        #expect(try await client.threads().map(\.members) == [["chief", "am"]])
    }

    @Test internal func aMemberMovesInRoomOrder() async throws {
        let server = CannedServer(["/threads/r1/members": [Answer(headers: json, chunks: [#"["am","chief"]"#])]])
        #expect(try await server.client().seat("r1", persona: "am", place: 0) == ["am", "chief"])
        #expect(
            try server.decoded("/threads/r1/members", as: [String: JSONValue].self)
                == ["persona": .string("am"), "at": .number(0), "remove": .bool(false)]
        )
    }

    @Test internal func aRoomsTurnsAndItsEndAreEvents() async throws {
        let server = CannedServer([
            "/threads/r1/events": [
                Answer(
                    headers: sse,
                    chunks: [
                        "id: b.1\nevent: room-turn\ndata: {\"member\":\"am\",\"reason\":\"round\"}\n\n",
                        "id: b.2\nevent: item\ndata: {\"id\":3,\"at\":1,\"kind\":\"agent\",\"text\":\"Hi\",",
                        "\"speaker\":\"am\"}\n\nid: b.3\nevent: room-stop\ndata: {\"reason\":\"done\"}\n\n",
                        "id: b.4\nevent: reset\ndata: {}\n\n",
                    ]
                )
            ]
        ])
        var got: [StreamEvent] = []
        for try await event in server.client().events("r1", since: "b.0") { got.append(event) }
        let reply = APIItem(id: 3, time: 1, kind: "agent", text: "Hi", speaker: "am")
        #expect(got == [.roomTurn("am"), .item(reply), .roomStop("done"), .reset])
    }
}
