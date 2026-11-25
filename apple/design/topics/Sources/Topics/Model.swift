import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct Topic: Identifiable, Sendable, Equatable {
    public let id: String
    public let title: String
    public let folder: String?
    public let last: String
    public let ago: String
    public var unread = 0
    public var working = false
    public var archived = false
}

internal enum Story {
    internal static let conductor = Fixtures.persona("cond")
    internal static let coder = Fixtures.persona("coder")
    internal static let researcher = Fixtures.persona("res")

    internal static let topics: [Topic] = [
        Topic(
            id: "c1", title: "cooee queue", folder: "cooee", last: "#504 design is up for your review", ago: "now",
            working: true
        ),
        Topic(
            id: "c2", title: "herdsman release", folder: "herdsman", last: "v0.31 tagged; notes in CHANGELOG.md",
            ago: "14m", unread: 2
        ),
        Topic(id: "c3", title: "Studio disk", folder: nil, last: "Freed 41 GB of build folders", ago: "2h"),
        Topic(
            id: "c4", title: "Lesson digest", folder: "cooee", last: "3 rev FAILs on Swift line caps", ago: "1d",
            unread: 1
        ),
        Topic(id: "c5", title: "Fleet hosts", folder: "herdsman", last: "lab is back online", ago: "3d"),
    ]

    internal static let archived: [Topic] = [
        Topic(
            id: "c8", title: "Bonjour pairing", folder: "cooee", last: "#128 merged", ago: "9d", archived: true
        ),
        Topic(
            id: "c9", title: "Timer install", folder: "herdsman", last: "drover timer is current", ago: "12d",
            archived: true
        ),
    ]

    internal static let research: [Topic] = [
        Topic(id: "r1", title: "Kestrel filings", folder: nil, last: "Allow fetch sec.gov?", ago: "12m", unread: 1),
        Topic(id: "r2", title: "Heat pumps EU", folder: nil, last: "Sources in notes/heat.md", ago: "1d"),
    ]

    internal static let items: [Item] = [
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
}
