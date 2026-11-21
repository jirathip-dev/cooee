@testable import CooeeAPI
@testable import CooeeCore
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
        #expect(looks.schemes.count == 8 && looks.schemes["paper"]?.label == "Paper")
        #expect(looks.schemes["paper"]?.mode == "light" && looks.schemes["github-dark"]?.mode == "dark")
        let mini = try #require(URL(string: "http://mini.local:4100"))
        #expect(looks.art("kitchen", on: mini)?.absoluteString == "http://mini.local:4100/look/kitchen/")
        #expect(looks.art("garden", on: mini)?.appending(path: "scene.svg").path == "/look/garden/scene.svg")
        #expect(looks.art("farm", on: mini) == nil && looks.art("plain", on: mini) == nil)
        #expect(looks.art("sea", on: mini) == nil)
    }

    @Test internal func theEditorChecksColoursAndSavesAPackByName() async throws {
        let report = #"{"dark": {"fails": [], "marks": {}}, "light": {"fails": ["x"], "marks": {}}}"#
        let server = CannedServer([
            "/api/v1/themes/check": [Answer(chunks: [report])],
            "/api/v1/themes/tidepool": [Answer(chunks: [#"{"written":true}"#]), Answer(status: 400, chunks: ["no"])],
        ])
        let client = server.client()
        let reports = try await client.checkTheme(dark: ["--bg": "#000000"], light: [:])
        #expect(reports["light"]?.fails == ["x"] && reports["dark"] == GateReport())
        let sent = try server.decoded("/api/v1/themes/check", as: [String: [String: String]].self)
        #expect(sent == ["dark": ["--bg": "#000000"], "light": [:]])
        let draft = ThemeDraft(from: "farm", label: "Tidepool", style: "scene", dark: [:], light: [:])
        try await client.saveTheme("tidepool", draft)
        #expect(server.requests("/api/v1/themes/tidepool").map(\.httpMethod) == ["PUT"])
        #expect(try server.decoded("/api/v1/themes/tidepool", as: ThemeDraft.self) == draft)
        await #expect(throws: APIError.self) { try await client.saveTheme("tidepool", draft) }
    }

    @Test internal func theEditorReadsAPacksArtAsText() async throws {
        let server = CannedServer(["/look/tidepool/sprites.svg": [Answer(chunks: ["<svg/>"])]])
        #expect(try await server.client().themeArt("tidepool", "sprites.svg") == "<svg/>")
        #expect(server.requests("/look/tidepool/sprites.svg").map(\.httpMethod) == ["GET"])
    }
}
