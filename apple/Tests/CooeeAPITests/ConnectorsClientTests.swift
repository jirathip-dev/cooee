@testable import CooeeAPI
import CooeeCore
import Foundation
import Testing

internal struct ConnectorsClientTests {
    private let said = Answer(headers: json, chunks: [#""ok""#])

    @Test internal func aSignInComesBackToThisClientsOwnOriginAndEndsAsItsFlowSays() async throws {
        let started = #"{"flow": "f1", "open": "https://linear.app/oauth/authorize?x=1", "until": "cooee"}"#
        let flow = #"{"state": "signed_in", "connector": "linear", "account": "main", "why": null}"#
        let server = CannedServer([
            "/connectors": [said, Answer(headers: json, chunks: [#"{"catalogue": [], "connectors": []}"#])],
            "/connectors/linear/signin": [Answer(headers: json, chunks: [started])],
            "/connectors/flows/f1": [Answer(headers: json, chunks: [flow])],
        ])
        let client = server.client()
        try await client.addConnector("linear")
        let start = try await client.signIn("linear", account: "main")
        #expect(start.flow == "f1" && start.until == "cooee" && start.help == nil)
        #expect(try await client.signInFlow("f1").signedIn)
        #expect(try await client.connectors() == ConnectorList())
        #expect(try server.decoded("/connectors", as: [String: String].self) == ["catalogue": "linear"])
        let host = try #require(client.base.host())
        let sent = try server.decoded("/connectors/linear/signin", as: [String: String].self)
        #expect(sent == ["account": "main", "origin": "http://\(host):4100"])
    }

    @Test internal func aTokenSignOutAndRemoveEachGoToTheirConnector() async throws {
        let server = CannedServer([
            "/connectors/github/token": [said], "/connectors/github/signout": [said], "/connectors/github": [said],
        ])
        let client = server.client()
        try await client.paste("github", flow: "f2", token: "ghp_x")
        try await client.signOut("github", account: "main")
        try await client.removeConnector("github")
        #expect(
            try server.decoded("/connectors/github/token", as: [String: String].self) == [
                "flow": "f2", "value": "ghp_x",
            ])
        #expect(try server.decoded("/connectors/github/signout", as: [String: String].self) == ["account": "main"])
        #expect(server.requests("/connectors/github").map(\.httpMethod) == ["DELETE"])
    }
}
