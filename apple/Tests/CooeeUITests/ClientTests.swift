import Foundation
import Synchronization
import Testing
@testable import CooeeUI

/// Canned server answers by path, through the client's real URLSession (a URLProtocol): each answer is a status,
/// headers and its body in chunks; requests are recorded.
final class Canned: URLProtocol, @unchecked Sendable {
    struct Answer { var status = 200; var headers: [String: String] = [:]; var chunks: [String] }
    static let answers = Mutex<[String: [Answer]]>([:])
    static let seen = Mutex<[URLRequest]>([])

    static func reset(_ a: [String: [Answer]]) { answers.withLock { $0 = a }; seen.withLock { $0 = [] } }
    static func requests(_ path: String) -> [URLRequest] { seen.withLock { $0.filter { $0.url?.path == path } } }

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }
    override func startLoading() {
        var r = request
        if let s = r.httpBodyStream {  // URLSession moves the body into a stream
            s.open(); var d = Data(); var buf = [UInt8](repeating: 0, count: 4096)
            while s.hasBytesAvailable { let n = s.read(&buf, maxLength: buf.count); if n <= 0 { break }; d.append(buf, count: n) }
            r.httpBody = d
        }
        Self.seen.withLock { $0.append(r) }
        let path = request.url!.path
        // The last answer for a path repeats.
        let a = Self.answers.withLock { q -> Answer? in
            guard var list = q[path], !list.isEmpty else { return nil }
            let first = list.count > 1 ? list.removeFirst() : list[0]
            q[path] = list
            return first
        } ?? Answer(status: 404, chunks: [])
        let resp = HTTPURLResponse(url: request.url!, statusCode: a.status, httpVersion: "HTTP/1.1", headerFields: a.headers)!
        client?.urlProtocol(self, didReceive: resp, cacheStoragePolicy: .notAllowed)
        for c in a.chunks { client?.urlProtocol(self, didLoad: Data(c.utf8)) }
        client?.urlProtocolDidFinishLoading(self)
    }
    override func stopLoading() {}

    static func client(token: String? = "tok") -> Client {
        let c = URLSessionConfiguration.ephemeral
        c.protocolClasses = [Canned.self]
        return Client(base: URL(string: "http://mini.local:4100")!, token: token, configuration: c, retry: .zero)
    }
}

let json = ["Content-Type": "application/json"]
let sse = ["Content-Type": "text/event-stream"]

// What the server answers (cooee-core's Summary, Persona, Item and Turn as serde writes them).
let threadsJSON = """
[{"id":"a1b2","title":"Flaky e2e","agent":"fake-acp","account":"","profile":"coder","persona":"coder","folder":"/w/project",
"state":"working","last":"Hello","at":1762790000000,"unread":true,"unseen":2,"archived":false},
{"id":"old","title":"Old","agent":"fake-acp","account":"","profile":"","persona":"","folder":"/w","state":"","last":"",
"at":1762700000000,"unread":false,"unseen":0,"archived":true,"workflow":{"source":"drover","group":"cooee","item":"#1",
"role":"impl","round":"1","url":"https://example.com"}}]
"""
let messagesJSON = """
{"items":[{"id":1,"at":1762790000000,"seq":0,"kind":"you","text":"Fix it","random_id":"r1","files":["1-0-log.txt"]},
{"id":2,"at":1762790001000,"seq":3,"kind":"tool","call":"c1","title":"Read web/e2e/look.spec.ts","status":"completed","ended":1762790002000},
{"id":3,"at":1762790001500,"seq":4,"kind":"tool","call":"c2","title":"Run npx playwright","status":"in_progress"},
{"id":4,"at":1762790002000,"seq":5,"kind":"approval","tool":"Edit src/lib.rs","options":[{"id":"allow","name":"Allow","kind":"allow_once"}]}],
"skipped_before":0,"skipped_after":0,"turn":{"text":"Hel","state":"working"},"cursor":"boot.5",
"queue":[{"text":"Then open the PR","random_id":"q1"}],"waiting":null}
"""

@Suite(.serialized) struct ClientTests {
    @Test func decodesThreadsPersonasAndASlice() async throws {
        Canned.reset([
            "/threads": [.init(headers: json, chunks: [threadsJSON])],
            "/personas": [.init(headers: json, chunks: [#"[{"id":"coder","name":"Coder","character":"","instructions":"","folder":"/w","agent":"fake-acp","model":"gpt-5.5","effort":"","plugins":["hello"]}]"#])],
            "/threads/a1b2/messages": [.init(headers: json, chunks: [messagesJSON])],
        ])
        let c = Canned.client()
        let rows = try await c.threads()
        #expect(rows.map(\.id) == ["a1b2", "old"])
        #expect(rows[0].persona == "coder" && rows[0].unseen == 2 && rows[0].state == "working" && rows[1].archived)
        let personas = try await c.personas()
        #expect(personas == [APIPersona(id: "coder", name: "Coder", agent: "fake-acp", model: "gpt-5.5", effort: "", folder: "/w",
                                        character: "", instructions: "", plugins: ["hello"])])
        let s = try await c.messages("a1b2")
        #expect(s.cursor == "boot.5" && s.turn == Turn(text: "Hel", state: "working") && s.queue.map(\.random_id) == ["q1"])
        #expect(s.items.map(\.kind) == ["you", "tool", "tool", "approval"])
        // Every call carries the device's cookie.
        #expect(Canned.requests("/threads").first?.value(forHTTPHeaderField: "Cookie") == "cooee=tok")
    }

    @Test func aDroppedStreamReconnectsWithLastEventIDAndGetsWhatItMissed() async throws {
        Canned.reset(["/threads/a1b2/events": [
            .init(headers: sse, chunks: ["id: boot.6\nevent: text\ndata: {\"text\":\"lo\"}\n\n: keep-alive\n\n",
                                         "id: boot.7\nevent: item\ndata: {\"id\":5,\"at\":1,\"seq\":7,\"kind\":\"agent\",",
                                         "\"text\":\"Hello\",\"stop_reason\":\"end_turn\"}\n\n"]),
            // The server replays from the id the client last saw.
            .init(headers: sse, chunks: ["id: boot.8\nevent: state\ndata: {\"state\":\"done\"}\n\n",
                                         "id: boot.9\nevent: queue\ndata: []\n\nid: boot.9\nevent: reset\ndata: {}\n\n"]),
        ]])
        var got: [StreamEvent] = []
        for try await e in Canned.client().events("a1b2", since: "boot.5") { got.append(e) }
        #expect(got == [.text("lo"), .item(APIItem(id: 5, at: 1, kind: "agent", text: "Hello")), .state(Turn(state: "done")),
                        .queue([]), .reset])
        let reqs = Canned.requests("/threads/a1b2/events")
        #expect(reqs.count == 2)
        #expect(reqs[0].url?.query == "since=boot.5" && reqs[0].value(forHTTPHeaderField: "Last-Event-ID") == nil)
        #expect(reqs[1].value(forHTTPHeaderField: "Last-Event-ID") == "boot.7")
    }

    @Test func aRefusedStreamEnds() async throws {
        Canned.reset(["/threads/gone/events": [.init(status: 404, chunks: [])]])
        await #expect(throws: APIError(status: 404, text: "")) {
            for try await _ in Canned.client().events("gone", since: "") {}
        }
    }

    @Test @MainActor func aChatStreamsTheReplyThenLoadsAgainOnReset() async throws {
        let after = #"{"items":[{"id":1,"at":1,"kind":"you","text":"Fix it","random_id":"r1"},{"id":2,"at":2,"kind":"agent","text":"All done","stop_reason":"end_turn"}],"turn":{"text":"","state":"done"},"cursor":"boot2.1","queue":[]}"#
        Canned.reset([
            "/threads/a1b2/messages": [.init(headers: json, chunks: [messagesJSON]), .init(headers: json, chunks: [after])],
            "/threads/a1b2/read": [.init(headers: json, chunks: ["null"])],
            "/threads/a1b2/events": [
                .init(headers: sse, chunks: ["id: boot.6\nevent: text\ndata: {\"text\":\"lo **there**\"}\n\n",
                                             "id: boot.7\nevent: reset\ndata: {}\n\n"]),
                .init(headers: sse, chunks: [": idle\n\n"]),
            ],
        ])
        let m = ChatModel(Canned.client(), id: "a1b2")
        var seen: [[Item]] = []
        let run = Task { await m.open() }
        // Each slice and event lands on the main actor between these yields.
        for _ in 0..<5000 where m.items.last?.kind != "agent" || Canned.requests("/threads/a1b2/events").count < 2 {
            if m.live.contains("there"), seen.isEmpty { seen.append(chatItems(m.items, live: m.live)) }
            await Task.yield()
        }
        run.cancel()
        // The slice's reply so far grew by the text event, drawn last with its caret.
        let streaming = try #require(seen.first)
        guard case .agent(_, let blocks) = streaming.last else { Issue.record("no live reply"); return }
        #expect(blocks == [.text("Hello **there** ▍")])
        // After the reset, the newest slice: the finished reply, no live text, the turn done.
        #expect(m.items.map(\.id) == [1, 2] && m.live == "" && !m.working)
        #expect(Canned.requests("/threads/a1b2/messages").count >= 2)
        // The second stream starts from the new slice's cursor.
        #expect(Canned.requests("/threads/a1b2/events").dropFirst().first?.url?.query == "since=boot2.1")
        // The read mark moved to the newest item.
        let read = Canned.requests("/threads/a1b2/read").first?.httpBody.map { String(decoding: $0, as: UTF8.self) }
        #expect(read == #"{"item":4}"#)
    }

    @Test @MainActor func sendingQueuesWhileATurnRuns() async throws {
        Canned.reset([
            "/threads/a1b2/send": [.init(headers: json, chunks: ["{}"])],
            "/threads/a1b2/queue": [.init(headers: json, chunks: ["[]"])],
        ])
        let m = ChatModel(Canned.client(), id: "a1b2")
        await m.send("first", files: [Attachment(name: "a.txt", bytes: Data("hi".utf8))])
        m.apply(.state(Turn(state: "working")))
        await m.send("second", files: [])
        let sent = try #require(Canned.requests("/threads/a1b2/send").first?.httpBody)
        let body = try JSONSerialization.jsonObject(with: sent) as! [String: Any]
        #expect(body["text"] as? String == "first")
        #expect((body["files"] as? [[String: String]]) == [["name": "a.txt", "data": "aGk="]])
        #expect(Canned.requests("/threads/a1b2/queue").count == 1)
    }

    @Test func aCodePairsAndAClaimWaitsThenPairs() async throws {
        Canned.reset([
            "/pair": [.init(status: 303, headers: ["Location": "/?paired", "Set-Cookie": "cooee=abc123; Path=/; HttpOnly"], chunks: [])],
            "/pair/request": [.init(headers: json, chunks: [#"{"word":"maple","claim":"cl1"}"#])],
            "/pair/claim": [.init(status: 202, headers: ["Cooee-Error": "1"], chunks: ["waiting for approval"]),
                            .init(headers: ["Set-Cookie": "cooee=def456; Path=/"], chunks: [])],
        ])
        let c = Canned.client(token: nil)
        #expect(try await c.pair(code: "123 456") == "abc123")
        let body = Canned.requests("/pair").first?.httpBody.map { String(decoding: $0, as: UTF8.self) }
        #expect(body == "code=123456")
        let ask = try await c.askToPair()
        #expect(ask.word == "maple")
        #expect(try await c.claim(ask.claim) == .waiting)
        #expect(try await c.claim(ask.claim) == .paired("def456"))
    }

    @Test func aWrongCodeAndADeclineSayWhy() async throws {
        Canned.reset([
            "/pair": [.init(status: 303, headers: ["Location": "/pair?bad=wrong&left=4"], chunks: [])],
            "/pair/claim": [.init(status: 403, headers: ["Cooee-Error": "1"], chunks: ["mini said no."])],
        ])
        let c = Canned.client(token: nil)
        await #expect(throws: APIError(status: 303, text: "That code is wrong.")) { try await c.pair(code: "000000") }
        #expect(try await c.claim("cl1") == .declined("mini said no."))
    }
}

@Suite struct DrawingTests {
    @Test func toolsFoldCardsWaitAndMarkdownSplits() throws {
        let s = try JSONDecoder().decode(Slice.self, from: Data(messagesJSON.utf8))
        let items = chatItems(s.items, live: "Done:\n\n```ts\nawait x();\n```\n- one\n- two")
        #expect(items.count == 4)
        guard case .you(1, "Fix it", ["1-0-log.txt"]) = items[0] else { Issue.record("\(items[0])"); return }
        guard case .tools(2, let rows) = items[1] else { Issue.record("\(items[1])"); return }
        #expect(rows.map(\.status) == ["done", "running"])
        guard case .note(4, let note) = items[2] else { Issue.record("\(items[2])"); return }
        #expect(note.contains("Edit src/lib.rs"))
        guard case .agent(_, let blocks) = items[3] else { Issue.record("\(items[3])"); return }
        #expect(blocks == [.text("Done:"), .code(lang: "ts", "await x();"), .bullets(["one", "two ▍"])])
    }

    @Test func addressesAndLinks() {
        #expect(target("mini.local:4100")?.url.absoluteString == "https://mini.local:4100")
        let link = target("http://127.0.0.1:4100/pair?code=123456")
        #expect(link?.url.absoluteString == "http://127.0.0.1:4100" && link?.code == "123456")
        #expect(target("  ") == nil)
    }
}
