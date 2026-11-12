import CooeeCore
import Foundation
import Testing

internal struct WorkflowTests {
    private let now = Date(timeIntervalSince1970: 1_000_000)
    private let coder = APIPersona(id: "coder", name: "Coder", agent: "codex")
    private let conductor = APIPersona(id: "cond", name: "Conductor", agent: "codex", plugins: ["drover"])
    private let drover = PluginRow(
        name: "drover", version: "1", commit: "c", places: [], claims: ["drover"], permissions: PluginPermissions()
    )

    private func row(_ id: String, persona: String = "", state: String = "done", lane: WorkflowLabel? = nil)
        -> ThreadRow
    {
        ThreadRow(
            id: id, title: "T \(id)", agent: "fake-acp", persona: persona, state: state, last: "", updated: 999_000_000,
            unseen: 0, archived: false, workflow: lane
        )
    }

    private func lane(_ item: String, _ role: String, _ round: String = "1", source: String = "drover")
        -> WorkflowLabel
    {
        WorkflowLabel(source: source, group: "cooee", item: item, role: role, round: round)
    }

    private var rows: [ThreadRow] {
        [
            row("chat", persona: "coder"),
            row("impl", lane: lane("#33", "impl")),
            row("loose"),
            row("nightly", persona: "coder", lane: lane("#1", "run", source: "nightly")),
        ]
    }

    @Test internal func homeListsOnlyPersonasChatsAndALaneGoesToThePersonaItsClaimingPluginIsAttachedTo() {
        let lists = Listing.home(rows: rows, personas: [coder, conductor], plugins: [drover], now: now)
        #expect(lists.chats.map(\.id) == ["chat"])
        #expect(lists.workflow.mapValues { $0.map(\.id) } == ["cond": ["impl"]])
        #expect(lists.others.map(\.id) == ["loose", "nightly"])
        #expect(lists.all.count == rows.count)
    }

    @Test internal func aLaneNoPluginClaimsFallsIntoOthers() {
        let unclaimed = PluginRow(name: "drover", version: "1", commit: "c", places: [], permissions: .init())
        let lists = Listing.home(rows: rows, personas: [coder, conductor], plugins: [unclaimed], now: now)
        #expect(lists.chats.map(\.id) == ["chat"] && lists.workflow.isEmpty)
        #expect(lists.others.map(\.id) == ["impl", "loose", "nightly"] && lists.others.count == 3)
        let detached = Listing.home(rows: rows, personas: [coder], plugins: [drover], now: now)
        #expect(detached.others.map(\.id) == ["impl", "loose", "nightly"])
    }

    @Test internal func thePersonaWithAClaimingPluginHasAnEmptySheetBeforeAnyLane() {
        let lists = Listing.home(rows: [], personas: [coder, conductor], plugins: [drover], now: now)
        #expect(lists.workflow == ["cond": []])
    }

    @Test internal func theSheetHasOneRowPerIssueNeedsYouFirstWithItsRoundsOldestFirst() throws {
        let sessions = Listing.chats(
            rows: [
                row("fix31", state: "working", lane: lane("#31", "fix")),
                row("ready30", state: "done", lane: lane("#30", "impl")),
                row("rev33", state: "error", lane: lane("#33", "rev")),
                row("ask32", state: "blocked", lane: lane("#32", "rev", "2")),
                row("impl31", state: "done", lane: lane("#31", "impl")),
            ],
            personas: [], now: now
        )
        let issues = Workflow.issues(sessions)
        #expect(issues.map(\.id) == ["cooee #32", "cooee #33", "cooee #31", "cooee #30"])
        #expect(issues.map(\.needsYou) == [true, false, false, false])
        let fix = try #require(issues.first { $0.id == "cooee #31" })
        #expect(fix.latest.id == "fix31" && fix.rounds.map(\.id) == ["impl31", "fix31"])
        #expect(issues[0].latest.workflow?.stage == "rev 2")
        #expect(Workflow.groups(issues).map(\.name) == ["needs you", "failed", "working", "ready"])
        #expect(Workflow.groups(issues).map { $0.issues.map(\.id) }.joined().elementsEqual(issues.map(\.id)))
    }

    @Test internal func othersSplitsChatsWithNoPersonaFromEachUnclaimedSourceAndNamesNeitherByItsAgent() throws {
        let others = Listing.home(rows: rows, personas: [coder], plugins: [], now: now).others
        let sections = Workflow.others(others)
        #expect(
            sections.map(\.name) == [
                "chats · no persona", "drover · no plugin claims it", "nightly · no plugin claims it",
            ])
        #expect(sections.map { $0.chats.map(\.id) } == [["loose"], ["impl"], ["nightly"]])
        let loose = try #require(sections.first?.chats.first)
        #expect(loose.persona.name == "T loose" && loose.persona.id.isEmpty && loose.row.title == "fake-acp")
        let lane = try #require(sections[1].chats.first)
        #expect(lane.persona.name == "cooee #33 · impl 1" && lane.persona.api.agent == "drover")
        #expect(lane.persona.look.coat == "bay" && loose.persona.look.coat.isEmpty)
    }
}
