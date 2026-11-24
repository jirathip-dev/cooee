import CooeeCore
@testable import CooeeUI
import Foundation
import SwiftUI
import Testing

@Suite(.serialized)
internal struct TopicFrames {
    private static let now = Date(timeIntervalSince1970: 1_000_000_000)
    private static let conductor = Fixtures.persona("cond")

    private static func topic(
        _ id: String, _ title: String, _ folder: String?, _ last: String, _ age: Int64, unread: Int = 0,
        working: Bool = false, archived: Bool = false
    ) -> ThreadRow {
        ThreadRow(
            id: id, title: title, agent: "codex", persona: "cond", state: working ? "working" : "done", last: last,
            updated: (1_000_000_000 - age) * 1000, unseen: unread, archived: archived,
            folder: folder.map { "/home/user/src/\($0)" } ?? ""
        )
    }

    private static let topics = Topics([
        topic("c1", "cooee queue", "cooee", "#504 design is up for your review", 0, working: true),
        topic("c2", "herdsman release", "herdsman", "v0.31 tagged; notes in CHANGELOG.md", 840, unread: 2),
        topic("c3", "Studio disk", nil, "Freed 41 GB of build folders", 7200),
        topic("c4", "Lesson digest", "cooee", "3 rev FAILs on Swift line caps", 86400, unread: 1),
        topic("c5", "Fleet hosts", "herdsman", "box is back online", 259_200),
        topic("c8", "Bonjour pairing", "cooee", "#128 merged", 777_600, archived: true),
        topic("c9", "Timer install", "herdsman", "drover timer is current", 1_036_800, archived: true),
    ])

    private static let items: [Item] = [
        .you(id: 1, text: "How's cooee's queue?"),
        .tools(id: 2, rows: [ToolRow(.run, "drover board cooee", "done"), ToolRow(.run, "drover asks cooee", "done")]),
        .agent(
            id: 3,
            markdown: [
                .text("Three in flight on **cooee**:"),
                .bullets([
                    "#504 topics: **design review**, waiting on you", "#370 retitle: review 1, PASS so far",
                    "#432 pinned opens newest: impl, 40m in",
                ]),
                .text("#504's frames are up. Approve it and the build issues get filed."),
            ]
        ),
    ]

    private static func strip(_ current: String, pressed: String? = nil) -> TopicActions {
        var strip = TopicActions(topics: topics, current: current)
        strip.pressed = pressed
        strip.folder = "/home/user/src/cooee"
        strip.recent = ["/home/user/src/cooee", "/home/user/src/herdsman"]
        return strip
    }

    @MainActor
    internal static func chat(plugins: Bool = true, scrolled: Bool = false, pressed: String? = nil) -> ChatView {
        var actions = ChatActions()
        if plugins { (actions.places, actions.counts, actions.workflow) = (["Queue", "Events"], [6, 12], 1) }
        actions.topics = strip("c1", pressed: pressed)
        var view = ChatView(
            persona: conductor, items: items, working: false, since: now, queued: [], back: true, actions: actions
        )
        view.end = scrolled
        return view
    }

    @MainActor
    private static func new() -> ChatView {
        var actions = ChatActions()
        actions.topics = strip("new:cond")
        var view = ChatView(
            persona: conductor, items: [], working: false, since: now, queued: [], back: true, actions: actions
        )
        view.draft = "Plan the build issues for topics"
        return view
    }

    @MainActor
    private static func list() -> some View {
        ZStack(alignment: .top) {
            chat()
            Dim()
            TopicList(
                persona: conductor, topics: strip("c1"), now: now, swiped: "c4", renaming: "c2",
                named: "herdsman v0.31 release"
            )
            .padding(.top, 60)
        }
    }

    private static func personaRows() -> [ListedChat] {
        func topics(_ persona: String, _ count: Int, _ row: Row, folder: String?) -> ListedChat {
            var chat = ListedChat(row: row, persona: Fixtures.persona(persona))
            (chat.topics, chat.folder) = (count, folder)
            return chat
        }
        let conductor = Row(
            id: "c2", persona: "cond", title: "herdsman release", last: "v0.31 tagged; notes in CHANGELOG.md",
            ago: "14m", unseen: 3, state: .done
        )
        let researcher = Row(
            id: "r1", persona: "res", title: "Kestrel filings", last: "Allow fetch sec.gov?", ago: "12m", unseen: 1,
            state: .done
        )
        let chats = Fixtures.chats
        return [
            chats[0], topics("cond", 5, conductor, folder: "/home/user/src/herdsman"),
            topics("res", 2, researcher, folder: ""), chats[3], chats[4],
        ]
    }

    @MainActor
    private static func rows() -> some View {
        VStack(spacing: 0) {
            ForEach(personaRows()) { ChatRow(row: $0.row, persona: $0.persona, topics: $0.topics, folder: $0.folder) }
        }
    }

    @MainActor
    private static func home() -> some View {
        ZStack(alignment: .top) {
            Backdrop()
            VStack(spacing: 0) {
                TopBar(title: HomeScreen.bundled.title, personas: Fixtures.personas, actions: HomeActions())
                Scroller {
                    PinnedRow(pins: Fixtures.personas.map(Pin.persona), rows: Fixtures.rows).padding(.top, 6)
                    rows().padding(.top, 14)
                }
            }
        }
    }

    internal static func frame(
        _ name: String, _ style: String, _ palette: Palette, _ view: @escaping @MainActor @Sendable () -> some View
    ) -> Frame {
        Frame.phone("\(name)-\(style)-\(palette.dark ? "dark" : "light")", palette, view).filed("topics")
    }

    internal static func topics(_ style: String, _ palette: Palette) -> [Frame] {
        [
            frame("chat", style, palette) { chat() }, frame("new", style, palette) { new() },
            frame("list", style, palette) { list() }, frame("home-persona", style, palette) { home() },
        ]
    }

    internal static var all: [Frame] {
        topics("modern", .farmDark) + topics("modern", .farmLight) + [
            frame("scrolled", "modern", .farmDark) { chat(plugins: false, scrolled: true) },
            frame("press", "modern", .farmDark) { chat(pressed: "c2") },
        ]
    }

    @MainActor @Test(arguments: all)
    internal func matchesTheDesign(_ frame: Frame) throws { try frame.matchesItsDesign() }
}
