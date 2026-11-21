@testable import CooeeCore
import Foundation
import Testing

internal struct ConnectorTests {
    private static let listed = #"""
        {"catalogue": [
           {"name": "linear", "label": "Linear", "publisher": "Linear", "source": "catalogue", "host": "mcp.linear.app",
            "leaves": "Issue text.", "checked": "2026-11-20", "auth": "oauth", "enabled": false,
            "tools": {"read": ["get_issue"], "write": ["create_issue"], "never": []}, "accounts": []},
           {"name": "notion", "label": "Notion", "publisher": "Notion", "source": "catalogue", "host": "mcp.notion.com",
            "leaves": "Pages.", "checked": "2026-11-20", "auth": "oauth", "enabled": false,
            "tools": {"read": [], "write": [], "never": []}, "accounts": []}],
         "connectors": [
           {"name": "linear", "label": "Linear", "publisher": "Linear", "source": "catalogue", "host": "mcp.linear.app",
            "leaves": "Issue text.", "checked": "2026-11-20", "auth": "oauth", "enabled": true,
            "tools": {"read": ["get_issue"], "write": ["create_issue"], "never": []},
            "accounts": [
              {"name": "main", "state": "needs_sign_in", "why": "refused", "scopes": [], "since": 1, "personas": []},
              {"name": "work", "state": "signed_in", "why": null, "scopes": [], "since": 2, "personas": ["Coder"]}]},
           {"name": "files", "label": "files", "publisher": "", "source": "command", "host": "", "leaves": "",
            "checked": null, "auth": "none", "enabled": true, "tools": {"read": [], "write": [], "never": []},
            "accounts": []}]}
        """#

    private let list = try? JSONDecoder().decode(ConnectorList.self, from: Data(Self.listed.utf8))

    @Test internal func theListOffersOnlyEntriesNotAddedYetAndNamesTheRegistrysFirst() throws {
        let list = try #require(list)
        #expect(list.addable.map(\.id) == ["notion"])
        let linear = try #require(list.named("linear"))
        #expect(linear.accounts.count == 2 && list.named("notion")?.accounts.isEmpty == true)
        #expect(list.named("files")?.source == .command && list.named("nope") == nil)
        #expect(linear.signedIn.map(\.id) == ["work"] && list.named("files")?.signedIn.isEmpty == true)
    }

    @Test internal func aSignInShowsItsAccountAsSigningInAddingItWhenNew() throws {
        let linear = try #require(list?.named("linear"))
        let again = linear.signingIn("main")
        #expect(
            again.accounts.map(\.state) == [.signingIn, .signedIn] && again.accounts.map(\.id) == ["main", "work"])
        let added = linear.signingIn("account-3")
        #expect(added.accounts.map(\.id) == ["main", "work", "account-3"] && added.accounts.last?.state == .signingIn)
        #expect(linear.nextAccount == "account-3" && ConnectorList().addable.isEmpty)
        #expect(try #require(list?.named("notion")).nextAccount == "main")
    }

    @Test internal func aPersonasPickIsSetInPlaceDetachedOrAddedAndSavedOnlyWhenTouched() throws {
        var coder = APIPersona(id: "coder", name: "Coder", agent: "codex")
        let untouched = try JSONEncoder().encode(coder)
        #expect(String(bytes: untouched, encoding: .utf8)?.contains("mcp") == false)
        coder.pick("linear", account: "main")
        coder.pick("github", account: "main")
        coder.pick("linear", account: "work")
        #expect(coder.mcp == [MCPPick(server: "linear", account: "work"), MCPPick(server: "github", account: "main")])
        coder.pick("linear", account: nil)
        coder.pick("github", account: nil)
        let saved = try JSONDecoder().decode(APIPersona.self, from: try JSONEncoder().encode(coder))
        #expect(saved.mcp == [])
        let flow = #"{"state": "signed_in", "connector": "linear", "account": "main", "why": null}"#
        #expect(try JSONDecoder().decode(SignInFlow.self, from: Data(flow.utf8)).signedIn)
    }
}
