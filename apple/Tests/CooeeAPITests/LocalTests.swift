#if os(macOS)
    import CooeeAPI
    import CooeeCore
    import Foundation
    import Testing

    internal struct LocalTests {
        private static let fakeCooee = #"""
            #!/bin/sh
            case "$*" in
              pair) printf 'Scan it\ncode  123 456  (one use, 10 minutes)\nlink  http://127.0.0.1:4747/pair?code=123456\n' ;;
              "pair asks") printf '[{"id":"ab12cd34","word":"maple fox","label":"iPhone","at":1}]\n' ;;
              "pair approve ab12cd34" | "pair decline ab12cd34") echo "$*" >>"$(dirname "$0")/answers" ;;
              "accounts ls --json") printf '[{"provider":"claude","name":"guy","signed_in":true,"folder":null}]\n' ;;
              *) echo "no request $3 is waiting" >&2; exit 1 ;;
            esac
            """#

        private let dir = FileManager.default.temporaryDirectory.appending(path: "cooee-local-\(UUID().uuidString)")

        private func fake(_ script: String) throws -> LocalServer {
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            let cooee = dir.appending(path: "cooee")
            try script.write(to: cooee, atomically: true, encoding: .utf8)
            try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: cooee.path())
            return LocalServer(cooee: cooee)
        }

        @Test internal func theMacReadsTheCodeAndRequestsAndAnswersThroughCooee() throws {
            let server = try fake(Self.fakeCooee)
            let code = try server.code()
            #expect(code.code == "123456" && code.link.absoluteString == "http://127.0.0.1:4747/pair?code=123456")
            #expect(try server.requests() == [PairRequest(id: "ab12cd34", word: "maple fox", label: "iPhone")])
            try server.answer("ab12cd34", yes: true)
            try server.answer("ab12cd34", yes: false)
            let answers = try String(contentsOf: dir.appending(path: "answers"), encoding: .utf8)
            #expect(answers == "pair approve ab12cd34\npair decline ab12cd34\n")
            #expect(try server.signIns() == [SignIn(provider: "claude", name: "guy", signedIn: true)])
            #expect(throws: APIError(status: 1, text: "no request gone is waiting")) {
                try server.answer("gone", yes: true)
            }
        }

        @Test internal func aPairWithoutALinkIsAnError() throws {
            let server = try fake("#!/bin/sh\necho 'the code ran out'\n")
            #expect(throws: APIError(status: 0, text: "`cooee pair` printed no link.")) { try server.code() }
        }

        @Test internal func theShellRunsLikeTerminalAndFindsTheAgentsAndNetwork() throws {
            #expect(try Shell.login("printf ok") == "ok")
            #expect(Shell.agents().map(\.name) == ["claude", "codex", "hermes"])
            let network = Shell.network()
            #expect(!network.name.isEmpty && network.lan.hasPrefix(network.name))
        }

        @Test internal func theConfigKeepsItsTablesAndSetsTopLevelKeys() throws {
            let fresh = LocalConfig(file: dir.appending(path: "new/config.toml"))
            #expect(fresh.port == 4747 && fresh.strings("roots").isEmpty)
            try fresh.set("roots", ["~/Projects"])
            #expect(fresh.strings("roots") == ["~/Projects"])
            let config = LocalConfig(file: dir.appending(path: "tables.toml"))
            let text = "port = 4800\n\n[[plugins]]\nroots = [\"inside\"]\n"
            try text.write(to: config.file, atomically: true, encoding: .utf8)
            #expect(config.port == 4800 && config.value("roots") == nil)
            try config.set("roots", ["~/Projects"])
            try config.set("shed", "/Applications/Cooee.app/Contents/MacOS/shed")
            try config.set("roots", ["~/Projects", "/Users/guy/src"])
            #expect(config.strings("roots") == ["~/Projects", "/Users/guy/src"])
            let written = try String(contentsOf: config.file, encoding: .utf8)
            #expect(
                written == """
                    shed = "/Applications/Cooee.app/Contents/MacOS/shed"
                    roots = ["~/Projects","/Users/guy/src"]
                    port = 4800

                    [[plugins]]
                    roots = ["inside"]

                    """
            )
            #expect(LocalConfig.standard.file.path().hasSuffix("/.config/cooee/cooee/config.toml"))
        }
    }
#endif
