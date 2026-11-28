import CooeeCore
import SwiftUI

extension Fixtures {
    private static let question = "What did Kestrel report for Q3 revenue? I need the number and the filing date."
    private static let note = "Use the 10-Q, not the deck, if they differ."

    public static let asked: [Item] = [
        .you(id: 1, text: "Add Kestrel's Q3 numbers to the export summary."),
        .agent(id: 2, markdown: [.text(Markdown.inline("I'll ask Researcher for the figures."))]),
        .asked(
            id: 3,
            Ask(
                persona: "res", name: "Researcher", text: question, thread: "x1", outcome: .replied, seconds: 48,
                note: note
            )
        ),
        .agent(
            id: 4,
            markdown: [
                .text(
                    Markdown.inline(
                        "Q3 was **$4.1M** (10-Q, 6 Nov, per your note). It's in export/summary.md. I'll check the discount too."
                    ))
            ]
        ),
        .asked(
            id: 5,
            Ask(
                persona: "am", name: "Account Manager", text: "Is the 8% Kestrel discount final?", thread: "",
                outcome: .notAllowed
            )
        ),
        .agent(
            id: 6,
            markdown: [
                .text(
                    Markdown.inline(
                        "I can't reach Account Manager from here; its discount is the one open line in the summary."))
            ]
        ),
    ]

    public static let exchange = Exchange(
        asker: persona("coder").api, target: persona("res").api, from: "c1", title: "Export for Kestrel", depth: 1,
        chat: "t2"
    )

    public static let linked: [Item] = [
        .divider(id: 0, text: "Today 09:50 · ask_persona"),
        .said(id: 1, speaker: "coder", time: "asked · 09:50", markdown: [.text(Markdown.inline(question))]),
        .said(id: -2, speaker: "res", time: "09:50", markdown: []),
        .tools(id: 2, rows: [ToolRow(.search, "sec.gov", "done"), ToolRow(.fetch, "sec.gov", "done")]),
        .you(id: 4, text: note),
        .asked(
            id: 5,
            Ask(
                persona: "am", name: "Account Manager", text: "Does Kestrel's Q3 deck restate revenue?", thread: "x2",
                outcome: .replied, seconds: 21
            )
        ),
        .said(
            id: 6, speaker: "res", time: "replied · 09:51",
            markdown: [
                .text(
                    Markdown.inline(
                        "Q3 revenue was **$4.1M**, up 12% on Q2, from the 10-Q filed 6 Nov, as you asked. Their deck agrees."
                    ))
            ]
        ),
        .divider(id: 7, text: "returned to Coder with your note · 48s"),
    ]

    public static var exchangeChats: [ListedChat] {
        let row = { (id: String, persona: String, title: String, last: String, ago: String) in
            Row(id: id, persona: persona, title: title, last: last, ago: ago, unseen: 0, state: .done)
        }
        let coder = row("c1", "coder", "Export for Kestrel", "Q3 was $4.1M. It's in export/summary.md.", "1m")
        return [
            ListedChat(row: coder, persona: persona("coder")),
            ListedChat(
                row: row("x1", "res", "", "", "1m"), persona: persona("res"), asker: persona("coder"), from: "c1"),
            ListedChat(row: row("x2", "am", "", "", "2m"), persona: persona("am"), asker: persona("res"), from: "x1"),
        ]
    }
}

extension LinkedView {
    public static func researcher(wide: Bool = false) -> LinkedView {
        LinkedView(exchange: Fixtures.exchange, items: Fixtures.linked, back: !wide)
    }
}

extension Sidebar where Column == EmptyView, Detail == LinkedView {
    public static func linked() -> Sidebar<EmptyView, LinkedView> {
        Sidebar(
            chats: Fixtures.roomChats, selected: "x1", actions: HomeActions(), column: { EmptyView() },
            detail: { LinkedView.researcher(wide: true) }
        )
    }
}
