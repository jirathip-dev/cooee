import CooeeCore

extension Fixtures {
    public static let streaming: [Item] = [
        .you(id: 1, text: "The look sheet e2e fails one run in five. Find out why and fix it.", files: ["e2e-log.txt"]),
        .tools(
            id: 2,
            rows: [
                ToolRow("Read web/e2e/look.spec.ts", "done"),
                ToolRow("Run npx playwright test look --repeat-each 10", "done"),
                ToolRow("Read web/ts/look.ts", "done"),
            ]
        ),
        .agent(
            id: 3,
            markdown: [
                .text(
                    "It's a race: the sheet animates in for **240 ms**, and the test taps *Scene* before the button is hit-testable."
                ),
                .code(
                    lang: "ts",
                    "await page.getByRole('dialog').waitFor();\nawait expect(sheet).toHaveCSS('opacity', '1');\nawait page.getByText('Scene').click();"
                ),
                .text("I'm adding the wait and running it 50 times ▍"),
            ]
        ),
        .tools(
            id: 4,
            rows: [
                ToolRow("Edit web/e2e/look.spec.ts", "done"),
                ToolRow("Run npx playwright test look --repeat-each 50", "running"),
            ]
        ),
    ]

    public static let cards: [Item] = [
        .you(id: 1, text: "Pull Kestrel's last three 10-Q filings and summarise the revenue lines."),
        .agent(id: 2, markdown: [.text("I need the web to reach EDGAR. Three things first:")]),
        card(
            3, nil,
            #"""
            {"type": "card", "id": "3", "kind": "ask", "title": "Allow fetch sec.gov?",
             "subtitle": "fetch https://www.sec.gov/cgi-bin/browse-edgar?company=kestrel",
             "blocks": [{"type": "choice", "id": "3-answer", "action": "answer", "recommended": "once",
               "options": [{"value": "once", "label": "Allow once"}, {"value": "always", "label": "Always for sec.gov"},
                           {"value": "deny", "label": "Deny"}]}]}
            """#
        ),
    ]

    public static let moreCards: [Item] = [
        .agent(
            id: 7, markdown: [.text("Allowed. The edgar plugin needs its settings, its key, and a run to install:")]),
        card(
            4, "edgar",
            #"""
            {"type": "card", "id": "4", "kind": "config", "title": "EDGAR plugin settings",
             "blocks": [{"type": "config", "id": "edgar", "submit": {"label": "Save", "action": "save-config"}, "fields": [
               {"type": "text", "name": "user_agent", "label": "User agent", "value": "cooee research"},
               {"type": "picker", "name": "rate", "label": "Rate", "value": "5 / s", "options": ["1 / s", "5 / s"]}]}]}
            """#
        ),
        card(
            5, "edgar",
            #"""
            {"type": "card", "id": "5", "kind": "secret", "title": "EDGAR_API_KEY",
             "blocks": [{"type": "secret", "id": "EDGAR_API_KEY", "name": "EDGAR_API_KEY", "action": "set-secret"}]}
            """#
        ),
        card(
            6, nil,
            #"""
            {"type": "card", "id": "6", "kind": "run", "title": "Install the edgar plugin's Python deps",
             "blocks": [{"type": "run", "id": "r7", "action": "run-card",
               "command": "uv pip install -r plugins/edgar/requirements.txt", "sha": "3b7e0c55a1d9", "dir": "~/research",
               "touches": "~/research/.venv only, no network after", "until": "\#(Date.now.addingTimeInterval(690).ISO8601Format())"}]}
            """#
        ),
    ]

    public static let conductor: [Item] = [
        .you(id: 1, text: "How's cooee's queue?"),
        .tools(id: 2, rows: [ToolRow("Run herdsman board cooee", "done"), ToolRow("Run herdsman asks cooee", "done")]),
        .agent(
            id: 3,
            markdown: [
                .text("Three in flight on **cooee**:"),
                .bullets([
                    "#124 native v1: **design review**, waiting on you", "#126 pairing API: review 1, PASS so far",
                    "#128 Bonjour: impl, 40m in",
                ]),
                .text("#124's screens are up. Approve it and impl starts on the same branch."),
            ]
        ),
        card(
            4, nil,
            #"""
            {"type": "card", "id": "4", "kind": "ask", "title": "#124 design: native app v1 screens",
             "subtitle": "38 frames: home, chat, cards, persona sheet, avatar menu, pairing, Mac setup, three styles",
             "blocks": [{"type": "choice", "id": "4-answer", "action": "answer", "recommended": "approve",
               "options": [{"value": "approve", "label": "Approve"}, {"value": "revise", "label": "Revise…"},
                           {"value": "reject", "label": "Reject"}]}]}
            """#
        ),
    ]

    private static func card(_ id: Int, _ poster: String?, _ json: String) -> Item {
        let block = try? JSONDecoder().decode(JSONValue.self, from: Data(json.utf8))
        return .card(id: id, poster: poster, block: block ?? .null)
    }
}
