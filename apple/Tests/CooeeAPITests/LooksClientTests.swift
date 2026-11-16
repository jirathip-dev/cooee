@testable import CooeeAPI
import Foundation
import Testing

internal struct LooksClientTests {
    @Test internal func theLooksReadEachPacksSchemesAndSprites() async throws {
        let server = CannedServer(["/api/v1/looks": [try Fixture.answer("looks.json")]])
        let looks = try await server.client().looks()
        #expect(looks.themes.keys.sorted() == ["farm", "garden", "kitchen", "plain"])
        #expect(looks.tokens("kitchen", dark: true)?["--bg"] == "#1a1816")
        #expect(looks.tokens("garden", dark: false)?["--bg"] == "#f7f2e6")
        #expect(looks.tokens("farm", dark: false)?["--bg"] == "#f5efda")
        #expect(looks.tokens("sea", dark: true) == nil)
        let mini = try #require(URL(string: "http://mini.local:4100"))
        #expect(looks.art("kitchen", on: mini)?.absoluteString == "http://mini.local:4100/look/kitchen/")
        #expect(looks.art("garden", on: mini)?.appending(path: "scene.svg").path == "/look/garden/scene.svg")
        #expect(looks.art("farm", on: mini) == nil && looks.art("plain", on: mini) == nil)
        #expect(looks.art("sea", on: mini) == nil)
    }
}
