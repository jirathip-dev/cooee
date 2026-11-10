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
        .choice(
            id: 3,
            Choice(
                title: "Allow fetch sec.gov?",
                detail: "fetch https://www.sec.gov/cgi-bin/browse-edgar?company=kestrel",
                options: ["Allow once", "Always for sec.gov", "Deny"],
                recommended: 0
            )
        ),
    ]

    public static let moreCards: [Item] = [
        .agent(
            id: 7, markdown: [.text("Allowed. The edgar plugin needs its settings, its key, and a run to install:")]),
        .config(
            id: 4,
            plugin: "edgar",
            title: "EDGAR plugin settings",
            fields: [Field("User agent", "cooee research"), Field("Rate", "5 / s")]
        ),
        .secret(
            id: 5, plugin: "edgar", name: "EDGAR_API_KEY", why: "Sent to the edgar plugin only, never to the agent"),
        .run(
            id: 6,
            RunCard(
                why: "Install the edgar plugin's Python deps",
                command: "uv pip install -r plugins/edgar/requirements.txt",
                sha: "3b7e0c55a1d9",
                folder: "~/research",
                blast: "~/research/.venv only, no network after",
                lapses: "lapses in 11m"
            )
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
        .choice(
            id: 4,
            Choice(
                title: "#124 design: native app v1 screens",
                detail: "38 frames: home, chat, cards, persona sheet, avatar menu, pairing, Mac setup, three styles",
                options: ["Approve", "Revise…", "Reject"],
                recommended: 0
            )
        ),
    ]
}
