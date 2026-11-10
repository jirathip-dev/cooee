import CooeeCore

public enum Fixtures {
    public static let personas: [Persona] = [
        Persona(
            api: APIPersona(
                id: "chief", name: "Chief", agent: "Claude Code", model: "opus 5.5", effort: "high", folder: "~/work",
                character:
                    "Calm chief of staff. Short answers in plain words; one question at a time, never a list of them.",
                instructions:
                    "Keep renewals.md current. Ask Account Manager for dates; never email a customer yourself."
            ),
            look: PersonaLook(initials: "CH", symbol: "star.fill", tint: .lead, coat: "ch"),
            state: .done
        ),
        Persona(
            api: APIPersona(
                id: "coder", name: "Coder", agent: "Codex", model: "gpt-5.5", effort: "medium", folder: "~/src/cooee",
                character: "Terse engineer. Shows the diff, not the essay.",
                instructions: "Run the tests before you say done. Small commits."
            ),
            look: PersonaLook(
                initials: "CO", symbol: "chevron.left.forwardslash.chevron.right", tint: .cyan, coat: "gry"),
            state: .working
        ),
        Persona(
            api: APIPersona(
                id: "res", name: "Researcher", agent: "Claude Code", model: "sonnet 5.5", effort: "medium",
                folder: "~/research", character: "Careful; cites every source.",
                instructions: "Put findings in notes/, one file a company."
            ),
            look: PersonaLook(initials: "RE", symbol: "magnifyingglass", tint: .purple, coat: "blk"),
            state: .needs
        ),
        Persona(
            api: APIPersona(
                id: "am", name: "Account Manager", agent: "Claude Code", model: "haiku 5.5", effort: "low",
                folder: "~/crm", character: "Warm, exact about dates.", instructions: "The CRM export is crm.csv."
            ),
            look: PersonaLook(initials: "AM", symbol: "briefcase.fill", tint: .link, coat: "bay"),
            state: .idle
        ),
        Persona(
            api: APIPersona(
                id: "cond", name: "Conductor", agent: "Claude Code", model: "opus 5.5", effort: "high",
                folder: "~/src/herdsman", character: "Runs the fleet; asks the owner only what herdsman says to.",
                instructions: "herdsman's conductor for cooee and herdsman.", plugins: ["Queue", "Events"]
            ),
            look: PersonaLook(
                initials: "CD", symbol: "point.3.connected.trianglepath.dotted", tint: .green, coat: "wht"),
            state: .working
        ),
    ]

    public static let rows: [Row] = [
        Row(
            id: "t1", persona: "coder", title: "Flaky e2e on the look sheet", last: "$ cargo test -p cooee-core",
            ago: "now", unseen: 2, state: .working
        ),
        Row(
            id: "t2", persona: "res", title: "Kestrel filings", last: "Allow fetch sec.gov/cgi-bin/browse-edgar?",
            ago: "12m", unseen: 1, state: .needs
        ),
        Row(
            id: "t3", persona: "cond", title: "cooee queue", last: "#124 design is up for your review", ago: "20m",
            unseen: 3, state: .working
        ),
        Row(
            id: "t4", persona: "chief", title: "Renewals", last: "Every Monday at 09:00 I'll check the renewals.",
            ago: "2m", unseen: 0, state: .done
        ),
        Row(
            id: "t5", persona: "am", title: "Birch & Co pricing", last: "Due this month: Acme 14 Nov, Birch 21 Nov",
            ago: "1h", unseen: 0, state: .idle
        ),
        Row(
            id: "t6", persona: "chief", title: "Q3 board deck", last: "Slides 4–9 are in deck/q3.key.", ago: "2d",
            unseen: 0, state: .done
        ),
        Row(
            id: "t7", persona: "coder", title: "Bonjour discovery spike", last: "_cooee._tcp answers on the LAN.",
            ago: "3d", unseen: 0, state: .done
        ),
    ]

    public static var chats: [ListedChat] {
        rows.map { ListedChat(row: $0, persona: persona($0.persona)) }
    }

    public static let queued = ["Then open the PR against staging"]

    public static let chiefMemory = [
        Field("Dana Okafor runs Kestrel's finance team, since September", "from Chief · 2h"),
        Field("Renewals are tracked in renewals.md, one line each", "from you · 1d"),
        Field("Account Manager knows the CRM; ask it for dates and owners", "from Chief · 3d"),
    ]

    internal static let nearby = [
        NearbyRow(server: NearbyServer(name: "mini", address: "Guy's Mac mini · 192.0.2.20"), icon: "macmini"),
        NearbyRow(server: NearbyServer(name: "studio", address: "studio.local · 192.0.2.31"), icon: "macstudio"),
    ]

    public static func persona(_ id: String) -> Persona {
        personas.first { $0.id == id } ?? Persona(APIPersona(id: id, name: id, agent: ""))
    }
}
