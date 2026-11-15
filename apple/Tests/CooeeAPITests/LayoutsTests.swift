@testable import CooeeAPI
import CooeeCore
import Foundation
import Testing

internal struct LayoutsTests {
    private static let served = #"{"etag":"e1","layout":{"format":1,"root":{"type":"nav"}}}"#
    private static let nav: JSONValue = .object(["format": .number(1), "root": .object(["type": .string("nav")])])

    @Test internal func aLayoutIsAskedForByItsEtagAndUnchangedIsNil() async throws {
        let server = CannedServer([
            "/layout": [Answer(headers: json, chunks: [Self.served]), Answer(status: 304, chunks: [])]
        ])
        let client = server.client()
        #expect(try await client.layout(etag: "") == LayoutAnswer(etag: "e1", layout: Self.nav))
        #expect(try await client.layout(etag: "e1") == nil)
        #expect(server.requests("/layout").map { $0.value(forHTTPHeaderField: "If-None-Match") } == ["\"\"", "\"e1\""])
    }

    @Test internal func resetPostsAndAnswersTheNewLayout() async throws {
        let server = CannedServer(["/layout/reset": [Answer(headers: json, chunks: [Self.served])]])
        #expect(try await server.client().resetLayout().etag == "e1")
        #expect(server.requests("/layout/reset").first?.httpMethod == "POST")
    }

    @Test internal func layoutEventsAreChangedAndBrokenWithItsLine() async throws {
        let stream = """
            : open

            event: broken
            data: {"line":12,"col":3,"why":"unknown place \\"float\\""}

            event: other
            data: {}

            event: changed
            data: {"etag":"e2"}


            """
        let server = CannedServer(["/layout/events": [Answer(headers: sse, chunks: [stream])]])
        var seen: [LayoutEvent] = []
        for try await event in server.client().layoutEvents() { seen.append(event) }
        #expect(seen == [.broken(line: 12, why: #"unknown place "float""#), .changed(etag: "e2")])
    }

    @Test internal func refusedLayoutEventsThrowTheirStatus() async {
        let server = CannedServer(["/layout/events": [Answer(status: 401, chunks: [])]])
        await #expect(throws: APIError(status: 401, text: "")) {
            for try await _ in server.client().layoutEvents() {}
        }
    }

    @Test internal func theLastGoodLayoutIsKeptOnDiskUntilForgotten() throws {
        let file = FileManager.default.temporaryDirectory.appending(path: "layout-\(UUID().uuidString)/layout.json")
        let kept = LayoutFile(file: file)
        let answer = LayoutAnswer(etag: "e1", layout: Self.nav)
        #expect(kept.load() == nil)
        kept.save(answer)
        #expect(kept.load() == answer)
        kept.save(nil)
        #expect(kept.load() == nil)
        try FileManager.default.removeItem(at: file.deletingLastPathComponent())
    }
}
