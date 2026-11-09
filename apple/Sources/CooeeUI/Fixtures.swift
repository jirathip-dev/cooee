import Foundation

// Fixture data shaped like docs/api.md: `Row` for the chat list, `Item` for a thread's rows, `Turn` and `Queued` for
// the working pill and the queued messages. Personas carry what the persona sheet shows (#116's q888/q893/q894).
// Illustrative, as #116's mock data: no networking (#124).

public enum State: String, Sendable { case working, needs, done, idle, fail, merged }

public struct Persona: Identifiable, Sendable {
    public let id: String
    public let name: String
    public let initials: String
    public let symbol: String  // Modern's SF Symbol on its avatar
    public let tint: Tint
    public let coat: String  // Scene's horse coat (apple/design/sprites.mjs)
    public let state: State
    public let agent: String
    public let model: String
    public let effort: String
    public let folder: String
    public let character: String
    public let instructions: String
    public let plugins: [String]  // q954: a plugin attaches to a persona (Conductor: Queue + Events)

    /// The state's ASCII figure (web/src/look.rs ASCII, herdsman#335).
    public var figure: String {
        switch state {
        case .working: "~o="; case .needs: "(!)"; case .done: "\\o/"; case .idle: "-.-"; case .fail: "x_x"
        case .merged: "\\o/"
        }
    }
    /// The farm theme's word for the state (packs/themes/farm/theme.json).
    public var word: String {
        switch state {
        case .working: "on the trail"; case .needs: "at the gate"; case .done: "in the stable"; case .idle: "grazing"
        case .fail: "refused"; case .merged: "over the fence"
        }
    }
    var pose: String {
        switch state { case .working: "edit"; default: state.rawValue }
    }
}

/// `GET /threads`' Row, with its persona.
public struct Row: Identifiable, Sendable {
    public let id: String
    public let persona: String
    public let title: String
    public let last: String
    public let at: String
    public let unseen: Int
    public let state: State
}

/// `Item`, as the chat draws it.
public enum Item: Identifiable, Sendable {
    case you(id: Int, text: String, files: [String] = [])
    case agent(id: Int, markdown: [Block])
    case tools(id: Int, rows: [(title: String, status: String)])  // consecutive `tool` items fold into one row
    case choice(id: Int, title: String, detail: String, options: [String], recommended: Int)
    case config(id: Int, plugin: String, title: String, fields: [(String, String)])
    case secret(id: Int, plugin: String, name: String, why: String)
    case run(id: Int, why: String, command: String, sha: String, folder: String, blast: String, lapses: String)
    case note(id: Int, text: String)

    public var id: Int {
        switch self {
        case .you(let i, _, _), .agent(let i, _), .tools(let i, _), .choice(let i, _, _, _, _), .config(let i, _, _, _),
             .secret(let i, _, _, _), .run(let i, _, _, _, _, _, _), .note(let i, _): i
        }
    }
}

/// Agent markdown, as blocks.
public enum Block: Sendable {
    case text(String)
    case code(lang: String, String)
    case bullets([String])
}

public enum Fixtures {
    public static let personas: [Persona] = [
        Persona(id: "chief", name: "Chief", initials: "CH", symbol: "star.fill", tint: .l, coat: "ch", state: .done,
                agent: "Claude Code", model: "opus 5.5", effort: "high", folder: "~/work",
                character: "Calm chief of staff. Short answers in plain words; one question at a time, never a list of them.",
                instructions: "Keep renewals.md current. Ask Account Manager for dates; never email a customer yourself.",
                plugins: []),
        Persona(id: "coder", name: "Coder", initials: "CO", symbol: "chevron.left.forwardslash.chevron.right",
                tint: .c, coat: "gry", state: .working, agent: "Codex", model: "gpt-5.5", effort: "medium",
                folder: "~/src/cooee", character: "Terse engineer. Shows the diff, not the essay.",
                instructions: "Run the tests before you say done. Small commits.", plugins: []),
        Persona(id: "res", name: "Researcher", initials: "RE", symbol: "magnifyingglass", tint: .p, coat: "blk",
                state: .needs, agent: "Claude Code", model: "sonnet 5.5", effort: "medium", folder: "~/research",
                character: "Careful; cites every source.", instructions: "Put findings in notes/, one file a company.",
                plugins: []),
        Persona(id: "am", name: "Account Manager", initials: "AM", symbol: "briefcase.fill", tint: .link, coat: "bay",
                state: .idle, agent: "Claude Code", model: "haiku 5.5", effort: "low", folder: "~/crm",
                character: "Warm, exact about dates.", instructions: "The CRM export is crm.csv.", plugins: []),
        Persona(id: "cond", name: "Conductor", initials: "CD", symbol: "point.3.connected.trianglepath.dotted",
                tint: .g, coat: "wht", state: .working, agent: "Claude Code", model: "opus 5.5", effort: "high",
                folder: "~/src/herdsman", character: "Runs the fleet; asks the owner only what herdsman says to.",
                instructions: "herdsman's conductor for cooee and herdsman.", plugins: ["Queue", "Events"]),
    ]
    public static func persona(_ id: String) -> Persona { personas.first { $0.id == id }! }

    public static let rows: [Row] = [
        Row(id: "t1", persona: "coder", title: "Flaky e2e on the look sheet", last: "$ cargo test -p cooee-core",
            at: "now", unseen: 2, state: .working),
        Row(id: "t2", persona: "res", title: "Kestrel filings", last: "Allow fetch sec.gov/cgi-bin/browse-edgar?",
            at: "12m", unseen: 1, state: .needs),
        Row(id: "t3", persona: "cond", title: "cooee queue", last: "#124 design is up for your review",
            at: "20m", unseen: 3, state: .working),
        Row(id: "t4", persona: "chief", title: "Renewals", last: "Every Monday at 09:00 I'll check the renewals.",
            at: "2m", unseen: 0, state: .done),
        Row(id: "t5", persona: "am", title: "Birch & Co pricing", last: "Due this month: Acme 14 Nov, Birch 21 Nov",
            at: "1h", unseen: 0, state: .idle),
        Row(id: "t6", persona: "chief", title: "Q3 board deck", last: "Slides 4–9 are in deck/q3.key.",
            at: "2d", unseen: 0, state: .done),
        Row(id: "t7", persona: "coder", title: "Bonjour discovery spike", last: "_cooee._tcp answers on the LAN.",
            at: "3d", unseen: 0, state: .done),
    ]

    /// Coder, mid-turn: the reply streaming, tools folded, a queued message.
    public static let streaming: [Item] = [
        .you(id: 1, text: "The look sheet e2e fails one run in five. Find out why and fix it.", files: ["e2e-log.txt"]),
        .tools(id: 2, rows: [("Read web/e2e/look.spec.ts", "done"), ("Run npx playwright test look --repeat-each 10", "done"),
                             ("Read web/ts/look.ts", "done")]),
        .agent(id: 3, markdown: [
            .text("It's a race: the sheet animates in for **240 ms**, and the test taps *Scene* before the button is hit-testable."),
            .code(lang: "ts", "await page.getByRole('dialog').waitFor();\nawait expect(sheet).toHaveCSS('opacity', '1');\nawait page.getByText('Scene').click();"),
            .text("I'm adding the wait and running it 50 times ▍"),
        ]),
        .tools(id: 4, rows: [("Edit web/e2e/look.spec.ts", "done"), ("Run npx playwright test look --repeat-each 50", "running")]),
    ]
    public static let queued = ["Then open the PR against staging"]

    /// Researcher, waiting on cards: choice, config, secret and a run card.
    public static let cards: [Item] = [
        .you(id: 1, text: "Pull Kestrel's last three 10-Q filings and summarise the revenue lines."),
        .agent(id: 2, markdown: [.text("I need the web to reach EDGAR. Three things first:")]),
        .choice(id: 3, title: "Allow fetch sec.gov?", detail: "fetch https://www.sec.gov/cgi-bin/browse-edgar?company=kestrel",
                options: ["Allow once", "Always for sec.gov", "Deny"], recommended: 0),
    ]
    /// Researcher's next cards: config, secret and a run card.
    public static let moreCards: [Item] = [
        .agent(id: 7, markdown: [.text("Allowed. The edgar plugin needs its settings, its key, and a run to install:")]),
        .config(id: 4, plugin: "edgar", title: "EDGAR plugin settings", fields: [("User agent", "cooee research"), ("Rate", "5 / s")]),
        .secret(id: 5, plugin: "edgar", name: "EDGAR_API_KEY", why: "Sent to the edgar plugin only, never to the agent"),
        .run(id: 6, why: "Install the edgar plugin's Python deps", command: "uv pip install -r plugins/edgar/requirements.txt",
             sha: "3b7e0c55a1d9", folder: "~/research", blast: "~/research/.venv only, no network after", lapses: "lapses in 11m"),
    ]

    /// Conductor's chat: its plugins' Queue and Events chips in the header, an ask card at the end.
    public static let conductor: [Item] = [
        .you(id: 1, text: "How's cooee's queue?"),
        .tools(id: 2, rows: [("Run herdsman board cooee", "done"), ("Run herdsman asks cooee", "done")]),
        .agent(id: 3, markdown: [
            .text("Three in flight on **cooee**:"),
            .bullets(["#124 native v1: **design review**, waiting on you", "#126 pairing API: review 1, PASS so far",
                      "#128 Bonjour: impl, 40m in"]),
            .text("#124's screens are up. Approve it and impl starts on the same branch."),
        ]),
        .choice(id: 4, title: "#124 design: native app v1 screens",
                detail: "38 frames: home, chat, cards, persona sheet, avatar menu, pairing, Mac setup, three styles",
                options: ["Approve", "Revise…", "Reject"], recommended: 0),
    ]

    public static let chiefMemory = [
        ("Dana Okafor runs Kestrel's finance team, since September", "from Chief · 2h"),
        ("Renewals are tracked in renewals.md, one line each", "from you · 1d"),
        ("Account Manager knows the CRM; ask it for dates and owners", "from Chief · 3d"),
    ]

    /// Servers on the LAN (Bonjour, `_cooee._tcp`).
    public static let nearby = [("mini", "Guy's Mac mini · 192.168.1.20", "macmini"), ("studio", "studio.local · 192.168.1.31", "macstudio")]
}
