import CooeeCore
import Foundation
import Testing

@testable import CooeeAPI

internal struct EventsTests {
    @Test internal func aDroppedStreamReconnectsWithLastEventIDAndGetsWhatItMissed() async throws {
        let server = CannedServer([
            "/threads/a1b2/events": [
                Answer(
                    headers: sse,
                    chunks: [
                        "id: boot.6\nevent: text\ndata: {\"text\":\"lo\"}\n\n: keep-alive\n\n",
                        "id: boot.7\r\nevent: item\ndata: {\"id\":5,\"at\":1,\"seq\":7,\"kind\":\"agent\",",
                        "\"text\":\"Hello\",\"stop_reason\":\"end_turn\"}\n\nevent: unknown\ndata: {}\n\nretry\n\n",
                    ]
                ),
                Answer(
                    headers: sse,
                    chunks: [
                        "id: boot.8\nevent: state\ndata: {\"state\":\"done\"}\n\n",
                        "id: boot.9\nevent: queue\ndata: []\n\nid: boot.9\nevent: reset\ndata: {}\n\n",
                    ]
                ),
            ]
        ])
        var got: [StreamEvent] = []
        for try await event in server.client().events("a1b2", since: "boot.5") { got.append(event) }
        #expect(
            got == [
                .text("lo"), .item(APIItem(id: 5, at: 1, kind: "agent", text: "Hello")), .state(Turn(state: "done")),
                .queue([]), .reset,
            ]
        )
        let requests = server.requests("/threads/a1b2/events")
        #expect(requests.count == 2)
        #expect(requests[0].url?.query == "since=boot.5" && requests[0].value(forHTTPHeaderField: "Last-Event-ID") == nil)
        #expect(requests[1].value(forHTTPHeaderField: "Last-Event-ID") == "boot.7")
    }

    @Test internal func aRefusedStreamEnds() async throws {
        let server = CannedServer(["/threads/gone/events": [Answer(status: 404, chunks: [])]])
        await #expect(throws: APIError(status: 404, text: "")) {
            for try await _ in server.client().events("gone", since: "") {}
        }
    }

    @Test internal func aServerErrorOrABadEventIsRetried() async throws {
        let server = CannedServer([
            "/threads/a1b2/events": [
                Answer(status: 503, chunks: []),
                Answer(headers: sse, chunks: ["id: b.1\nevent: text\ndata: not json\n\n"]),
                Answer(headers: sse, chunks: ["id: b.2\nevent: reset\ndata: {}\n\n"]),
            ]
        ])
        var got: [StreamEvent] = []
        for try await event in server.client().events("a1b2", since: "b.0") { got.append(event) }
        #expect(got == [.reset])
        #expect(server.requests("/threads/a1b2/events").count == 3)
    }

    @Test internal func aCancelledStreamStops() async throws {
        let server = CannedServer(["/threads/a1b2/events": [Answer(headers: sse, chunks: [": idle\n\n"])]])
        let stream = server.client().events("a1b2", since: "")
        let reader = Task {
            var count = 0
            for try await _ in stream { count += 1 }
            return count
        }
        for _ in 0..<100_000 where server.requests("/threads/a1b2/events").count < 3 { await Task.yield() }
        reader.cancel()
        #expect(try await reader.value == 0)
    }

    @Test internal func sseFramesFollowTheSpec() throws {
        var parser = SSEParser(id: "start")
        #expect(parser.feed("") == nil)
        #expect(parser.feed(": comment") == nil)
        #expect(parser.feed("data: one") == nil)
        #expect(parser.feed("data:two") == nil)
        #expect(parser.feed("field") == nil)
        #expect(parser.feed("") == SSEFrame(id: "start", event: "message", data: "one\ntwo"))
        #expect(parser.feed("event: reset") == nil)
        #expect(parser.feed("") == SSEFrame(id: "start", event: "reset", data: ""))
        #expect(try Client.decode("message", Data("{}".utf8)) == nil)
    }
}
