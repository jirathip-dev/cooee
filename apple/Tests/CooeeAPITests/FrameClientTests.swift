@testable import CooeeAPI
import Foundation
import Testing

internal struct FrameClientTests {
    @Test internal func aReplayFrameIsItsBytesWithTheCookie() async throws {
        let server = CannedServer(["/threads/a1b2/frames/f1": [Answer(chunks: ["pixels"])]])
        #expect(try await server.client().frame("a1b2", "f1") == Data("pixels".utf8))
        #expect(server.requests("/threads/a1b2/frames/f1").first?.value(forHTTPHeaderField: "Cookie") == "cooee=tok")
    }
}
