import CooeeCore
@testable import CooeeUI
import Foundation
import SwiftUI

internal enum TopicFrames {
    private static let now = Date(timeIntervalSince1970: 1_000_000_000)
    private static let conductor = Fixtures.persona("cond")

    private static func topic(
        _ id: String, _ title: String, _ folder: String?, _ last: String, _ age: Int64, unread: Int = 0,
        working: Bool = false, archived: Bool = false
    ) -> ThreadRow {
        ThreadRow(
            id: id, title: title, agent: "codex", persona: "cond", state: working ? "working" : "done", last: last,
            updated: (1_000_000_000 - age) * 1000, unseen: unread, archived: archived,
            folder: folder.map { "/Users/me/src/\($0)" } ?? ""
        )
    }

    private static let topics = Topics([
        topic("c1", "cooee queue", "cooee", "#504 design is up for your review", 0, working: true),
        topic("c2", "herdsman release", "herdsman", "v0.31 tagged; notes in CHANGELOG.md", 840, unread: 2),
        topic("c3", "Studio disk", nil, "Freed 41 GB of build folders", 7200),
        topic("c4", "Lesson digest", "cooee", "3 rev FAILs on Swift line caps", 86400, unread: 1),
        topic("c5", "Fleet hosts", "herdsman", "air is back online", 259_200),
        topic("c8", "Bonjour pairing", "cooee", "#128 merged", 777_600, archived: true),
        topic("c9", "Timer install", "herdsman", "drover timer is current", 1_036_800, archived: true),
    ])

    private static let items: [Item] = [
        .you(id: 1, text: "How's cooee's queue?"),
        .tools(id: 2, rows: [ToolRow("Run drover board cooee", "done"), ToolRow("Run drover asks cooee", "done")]),
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
        strip.folder = "/Users/me/src/cooee"
        strip.recent = ["/Users/me/src/cooee", "/Users/me/src/herdsman"]
        return strip
    }

    @MainActor
    private static func chat(plugins: Bool = true, scrolled: Bool = false, pressed: String? = nil) -> ChatView {
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

    private static let every: [(LookStyle, Palette)] = [
        (.modern, .farmDark), (.modern, .farmLight), (.terminal, .farmDark), (.scene, .farmLight),
    ]

    private static func frame(
        _ name: String, _ style: LookStyle, _ palette: Palette, _ view: @escaping @MainActor @Sendable () -> some View
    ) -> Frame {
        Frame(
            name: "\(name)-\(style.rawValue)-\(palette.dark ? "dark" : "light")", width: 402, height: 874,
            palette: palette, style: style, phone: true, folder: "topics", view: { AnyView(view()) }
        )
    }

    internal static var all: [Frame] {
        every.flatMap { style, palette in
            [
                frame("chat", style, palette) { chat() }, frame("new", style, palette) { new() },
                frame("list", style, palette) { list() },
            ]
        } + [
            frame("scrolled", .modern, .farmDark) { chat(plugins: false, scrolled: true) },
            frame("press", .modern, .farmDark) { chat(pressed: "c2") },
            frame("press", .terminal, .farmLight) { chat(pressed: "c2") },
        ]
    }
}
