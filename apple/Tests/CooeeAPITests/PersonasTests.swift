@testable import CooeeAPI
import CooeeCore
import Foundation
import Testing

extension APIPersona {
    internal static var coder: APIPersona {
        var coder = APIPersona(
            id: "coder", name: "Coder", agent: "fake-acp", model: "gpt-5.5", effort: "high", folder: "/w",
            character: "Terse.", instructions: "Test first.", plugins: ["hello"]
        )
        coder.avatar = "data:image/png;base64,AA=="
        coder.machine = "air"
        coder.role = "orchestrator"
        coder.style = .detailed
        coder.fleets = ["cooee"]
        coder.canMessage = ["chief"]
        coder.pinned = true
        coder.profile = "coder"
        coder.version = "v1"
        return coder
    }
}

internal struct PersonasTests {
    private let persona = try? Fixture.text("personas.json").dropFirst().dropLast(2)

    @Test internal func anEditSavesEveryFieldThroughItsScreenSoNothingTheAppDoesNotDrawIsLost() async throws {
        let one = Answer(headers: json, chunks: [String(try #require(persona))])
        let server = CannedServer(["/screens/action": [one], "/personas": [one]])
        var edited = APIPersona.coder
        edited.character = "Terse, kind."
        _ = try await server.client().act(.core, screen: "/screens/persona/coder", .save(edited))
        let sent = try server.decoded("/screens/action", as: JSONValue.self)
        #expect(sent.string("screen") == "/screens/persona/coder" && sent.string("action") == "save")
        let values = try JSONEncoder().encode(try #require(sent["values"]))
        #expect(try JSONDecoder().decode(APIPersona.self, from: values) == edited)
        let body = try #require(server.body("/screens/action"))
        #expect(
            body.contains(#""can_message":["chief"]"#) && body.contains(#""machine":"air""#)
                && body.contains(#""pinned":true"#)
                && body.contains(#""role":"orchestrator""#) && body.contains(#""fleets":["cooee"]"#)
                && body.contains(#""style":"detailed""#) && body.contains(#""version":"v1""#))
        _ = try await server.client().create(APIPersona(id: "", name: "Coder", agent: "fake-acp"))
        #expect(server.requests("/personas").first?.httpMethod == "POST")
    }

    @Test internal func aDeleteRemovesThePersonaAndAModelPickGoesToItsThread() async throws {
        let server = CannedServer([
            "/personas/coder": [Answer(headers: json, chunks: ["null"])],
            "/threads/t1/model": [Answer(headers: json, chunks: [#""switched""#])],
            "/threads/t1/agent": [Answer(headers: json, chunks: ["{}"])],
        ])
        let client = server.client()
        try await client.remove("coder")
        try await client.model("t1", model: "opus", effort: "high")
        try await client.switchAgent("t1", to: "codex")
        #expect(try server.decoded("/threads/t1/agent", as: [String: String].self) == ["agent": "codex"])
        #expect(server.requests("/personas/coder").map(\.httpMethod) == ["DELETE"])
        #expect(
            try server.decoded("/threads/t1/model", as: [String: String].self) == ["model": "opus", "effort": "high"])
    }

    @Test internal func pushOnPostsTheDeviceTokenAndOffDeletesIt() async throws {
        let server = CannedServer(["/push/apns": [Answer(chunks: ["ok"])]])
        try await server.client().push("ab12", enabled: true)
        try await server.client().push("ab12", enabled: false)
        #expect(server.requests("/push/apns").map(\.httpMethod) == ["POST", "DELETE"])
        #expect(
            server.requests("/push/apns").map { $0.httpBody }
                == Array(repeating: Data(#"{"token":"ab12"}"#.utf8), count: 2))
    }

    @Test internal func aTimingIsPostedByNameAndMs() async throws {
        let server = CannedServer(["/perf": [Answer(chunks: ["ok"])]])
        try await server.client().perf("tap", millis: 42)
        let sent = try server.decoded("/perf", as: [String: JSONValue].self)
        #expect(sent == ["name": .string("tap"), "ms": .number(42)])
    }

    @Test internal func aReportPostsTheScreenTheElementAsTargetAndTheNote() async throws {
        let server = CannedServer(["/report": [Answer(chunks: ["report sent"])]])
        try await server.client().report(Report(note: "wrong time", path: "/t1", element: "working pill"))
        let sent = try server.decoded("/report", as: [String: JSONValue].self)
        #expect(sent == ["note": .string("wrong time"), "path": .string("/t1"), "target": .string("working pill")])
    }

    @Test internal func aRefusedSaveCarriesTheServersReason() async throws {
        let refused = Answer(status: 400, headers: ["Cooee-Error": "1"], chunks: ["a persona needs a name"])
        let server = CannedServer(["/personas": [refused]])
        await #expect(throws: APIError(status: 400, text: "a persona needs a name")) {
            try await server.client().create(APIPersona(id: "", name: "", agent: ""))
        }
    }

    @Test internal func theAgentChipsListsAndTheFoldersDecodeFromTheServer() async throws {
        let agents =
            #"[{"id":"fake-acp","efforts":["low","high"],"gaps":[]},{"id":"codex","efforts":[],"gaps":[{"kind":null,"words":"Reads yours."},{"kind":"run","words":"No ask."}]}]"#
        let models =
            #"[{"agent":"fake-acp","account":"","seen":5,"models":[["fake-opus-3","Opus"]],"short":{"opus":"fake-opus-3"}}]"#
        let folders = #"{"path":"/w/my app","home":"/home/user","dirs":["src"],"roots":["/w"]}"#
        let server = CannedServer([
            "/agents": [Answer(headers: json, chunks: [agents])],
            "/models": [Answer(headers: json, chunks: [models])],
            "/folders": [Answer(headers: json, chunks: [folders])],
        ])
        let client = server.client()
        #expect(
            try await client.agents() == [
                AgentOption(id: "fake-acp", efforts: ["low", "high"]),
                AgentOption(id: "codex", gaps: [Gap(words: "Reads yours."), Gap(kind: .run, words: "No ask.")]),
            ])
        let lists = try await client.models()
        #expect(
            lists == [ModelList(agent: "fake-acp", models: [["fake-opus-3", "Opus"]], short: ["opus": "fake-opus-3"])])
        #expect(ModelList.picks(lists, agent: "fake-acp") == ["opus", "fake-opus-3"])
        let listing = try await client.folders("/w/my app")
        #expect(listing == FolderListing(path: "/w/my app", dirs: ["src"], roots: ["/w"]))
        _ = try await client.folders(nil)
        let asked = server.requests("/folders").map { $0.url?.query() }
        #expect(asked == ["path=%2Fw%2Fmy%20app", nil])
    }

    @Test internal func rulesAreListedAddedAndRemovedWithTheServersWordsAndItsRefusal() async throws {
        let listed =
            #"[{"id":"3f","effect":"allow","kind":"run","pattern":"cargo test *","folder":"/w","by":"card","at":1795003200000,"words":"Allow running cargo test … in /w"}]"#
        let added =
            #"{"id":"9e","effect":"deny","kind":"fetch","pattern":"*.ads","by":"settings","at":1795176000000,"words":"Deny fetching from *.ads"}"#
        let refused = Answer(status: 400, headers: ["Cooee-Error": "1"], chunks: ["a rule needs a one-line pattern"])
        let server = CannedServer([
            "/personas/coder/rules": [
                Answer(headers: json, chunks: [listed]), Answer(headers: json, chunks: [added]), refused,
            ],
            "/personas/coder/rules/3f": [Answer(chunks: [])],
        ])
        let client = server.client()
        let rules = try await client.rules("coder")
        #expect(rules.map(\.words) == ["Allow running cargo test … in /w"])
        #expect(rules.first?.origin == "from a card · 18 Nov" && rules.first?.kind == .run)
        let rule = try await client.addRule("coder", NewRule(effect: .deny, kind: .fetch, pattern: "*.ads"))
        #expect(rule.origin == "added here · 20 Nov" && rule.effect == .deny)
        let posted = server.requests("/personas/coder/rules").compactMap(\.httpBody).first
        let sent = String(bytes: try #require(posted), encoding: .utf8) ?? ""
        #expect(sent.contains(#""effect":"deny""#) && sent.contains(#""kind":"fetch""#) && !sent.contains("folder"))
        await #expect(throws: APIError(status: 400, text: "a rule needs a one-line pattern")) {
            try await client.addRule("coder", NewRule(pattern: ""))
        }
        try await client.removeRule("coder", "3f")
        #expect(server.requests("/personas/coder/rules/3f").map(\.httpMethod) == ["DELETE"])
    }

    @Test internal func aNewThreadCarriesThePickedModelAndEffort() async throws {
        let server = CannedServer(["/threads": [Answer(headers: json, chunks: [#"{"id":"n1"}"#])]])
        let persona = APIPersona(id: "coder", name: "Coder", agent: "fake-acp", model: "opus", effort: "high")
        _ = try await server.client().newThread(persona: persona, machine: "", text: "hi", randomID: "r9")
        let sent = try server.decoded("/threads", as: [String: JSONValue].self)
        #expect(sent["model"] == .string("opus") && sent["effort"] == .string("high"))
    }

    @Test internal func theGlanceDecodesTheServersRowsAndEachActivityTokenIsPostedWithItsActivity() async throws {
        let row =
            #"{"thread":"a","title":"Kestrel","persona":"res","name":"Researcher","machine":"mini","#
            + #""line":"Allow fetch?","since":7,"lane":false}"#
        let glance = #"{"at":42,"asks":[\#(row)],"blocked":[],"running":[]}"#
        let posted = Answer(chunks: ["live activity on"])
        let server = CannedServer(["/glance": [Answer(headers: json, chunks: [glance])], "/push/activity": [posted]])
        let client = server.client()
        let got = try await client.glance()
        let ask = GlanceRow(
            thread: "a", title: "Kestrel", persona: "res", name: "Researcher", machine: "mini", line: "Allow fetch?",
            since: 7)
        #expect(got == Snapshot(time: 42, asks: [ask]))
        try await client.activity("ab12", for: nil)
        try await client.activity("cd34", for: "act-1")
        let bodies = try server.requests("/push/activity").map {
            try JSONDecoder().decode([String: String].self, from: try #require($0.httpBody))
        }
        #expect(bodies == [["token": "ab12"], ["token": "cd34", "activity": "act-1"]])
    }
}
