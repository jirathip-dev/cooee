@testable import CooeeAPI
import CooeeCore
import Foundation
import Testing

internal struct TerminalClientTests {
    @Test internal func listsStartsAndTypesThroughTheTerminalPlugin() async throws {
        let server = CannedServer([
            "/p/terminal/api/list": [
                Answer(headers: json, chunks: [#"[{"name":"owner-1","owner":true,"cmd":"zsh"}]"#])
            ],
            "/p/terminal/api/shell": [Answer(headers: json, chunks: [#"{"terminal":"owner-2"}"#])],
            "/p/terminal/api/input": [Answer(headers: json, chunks: ["{}"])],
            "/p/terminal/api/keys": [Answer(headers: json, chunks: ["{}"])],
            "/p/terminal/api/resize": [Answer(headers: json, chunks: ["{}"])],
        ])
        let client = server.client()
        #expect(try await client.terminals() == [TerminalShell(name: "owner-1", owner: true)])
        #expect(try await client.newShell() == "owner-2")
        try await client.type(Keystrokes("ls -la"), into: "owner-1")
        try await client.press(["enter"], in: "owner-1")
        try await client.resize("owner-1", to: try #require(TerminalSize(cols: 80, rows: 24)))
        let typed = try server.decoded("/p/terminal/api/input", as: [String: String].self)
        #expect(typed == ["terminal": "owner-1", "text": "ls -la"])
        #expect(
            try server.decoded("/p/terminal/api/keys", as: [String: JSONValue].self)["keys"]
                == .array([.string("enter")]))
        let size = try server.decoded("/p/terminal/api/resize", as: [String: JSONValue].self)
        #expect(size["cols"] == .number(80) && size["rows"] == .number(24))
    }

    @Test internal func watchingYieldsScreenUpdatesUntilTheShellCloses() async throws {
        let screen =
            #"{\"full\":true,\"rows\":1,\"cursor\":[0,2],\"lines\":[{\"row\":0,\"runs\":[{\"text\":\"$ \"}]}]}"#
        let server = CannedServer([
            "/p/terminal/api/watch": [
                Answer(
                    headers: sse,
                    chunks: [
                        "data: {\"event\":\"screen\",\"data\":\"\(screen)\"}\n\n",
                        "data: {\"event\":\"closed\",\"data\":\"{}\"}\n\n",
                        "data: {\"event\":\"screen\",\"data\":\"\(screen)\"}\n\n",
                    ])
            ]
        ])
        var updates: [TerminalUpdate] = []
        for try await update in server.client().watch("owner-1") { updates.append(update) }
        #expect(updates.map(\.rows) == [1])
        #expect(server.requests("/p/terminal/api/watch").first?.url?.query() == "t=owner-1")
    }
}
