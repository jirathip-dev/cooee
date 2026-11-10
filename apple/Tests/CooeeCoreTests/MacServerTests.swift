import CooeeCore
import Foundation
import Testing

internal struct MacServerTests {
    @Test internal func agentsFoundOnPathAreListedInOrderTheRestNotInstalled() {
        let found = FoundAgent.list(
            found: "codex\t~/.local/bin/codex\tcodex-cli 0.61.0\nclaude\t/opt/homebrew/bin/claude\t\n")
        #expect(found.map(\.title) == ["Claude Code", "Codex", "Hermes"])
        #expect(
            found.map(\.detail) == [
                "/opt/homebrew/bin/claude", "0.61.0 · ~/.local/bin/codex", "not installed · optional",
            ])
        #expect(FoundAgent.title("gemini") == "gemini")
    }

    @Test internal func aSignInSaysWhoOrThatItIsMissing() throws {
        let json =
            #"[{"provider":"claude","name":"guy","signed_in":true,"folder":null},"#
            + #"{"provider":"codex","name":"","signed_in":true},{"provider":"hermes","name":"x","signed_in":false}]"#
        let signIns = try JSONDecoder().decode([SignIn].self, from: Data(json.utf8))
        #expect(signIns.map(\.detail) == ["guy", "signed in", "not signed in"])
    }

    @Test internal func theTailnetNameComesFromTailscaleStatus() {
        #expect(
            Network.tailnet(status: Data(#"{"Self":{"DNSName":"mini.example.com."}}"#.utf8)) == "mini.example.com")
        #expect(Network.tailnet(status: Data(#"{"Self":{"DNSName":""}}"#.utf8)) == nil)
        #expect(Network.tailnet(status: Data("tailscale: command not found".utf8)) == nil)
    }

    @Test internal func cooeePairsOutputGivesTheLinkCodeAndLifetime() throws {
        let output =
            "Scan it\n█▀█\ncode  123 456  (one use, 10 minutes)\nlink  https://mini.example.com:4747/pair?code=123456\n"
        let code = try #require(PairCode(output: output))
        #expect(code.code == "123456" && code.minutes == 10 && code.host == "mini.example.com")
        #expect(code.local.absoluteString == "http://127.0.0.1:4747")
        #expect(PairCode(output: "link  http://127.0.0.1:4747/pair\n") == nil)
        #expect(PairCode(output: "the code ran out") == nil)
        let plain = try #require(PairCode(output: "link  http://127.0.0.1:4100/pair?code=000111"))
        #expect(plain.minutes == 10 && plain.local.port == 4100)
    }

    @Test internal func loginItemsSayHowEachRuns() {
        #expect(LoginItems(cooee: .enabled, shed: .missing).summary == "cooee on · shed not found")
        #expect(LoginItems(cooee: .off, shed: .enabled).summary == "cooee off · shed on")
    }
}
