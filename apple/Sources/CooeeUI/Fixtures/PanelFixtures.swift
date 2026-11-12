extension PanelContent {
    internal static let conductor = PanelContent(
        screen: "github.com/jirathip-dev/cooee/pulls",
        sites: 3,
        terminal: [
            "$ herdsman board cooee",
            "#244 design   live  2m",
            "#236 queued   after #244",
            "#229 review1  PASS  ready",
            "#213 merged",
        ],
        issues: [
            PanelIssue(number: 244, title: "Persona panel design", stage: "design", tone: .working),
            PanelIssue(number: 229, title: "Workflow sheet", stage: "ready · merge?", tone: .needs),
            PanelIssue(number: 217, title: "Persona history tool", stage: "merged", tone: .done),
        ],
        moreIssues: 4,
        routines: [
            PanelRoutine(name: "Morning board", when: "Weekdays 08:30", next: "next tomorrow 08:30", isOn: true),
            PanelRoutine(name: "Stale PR sweep", when: "When CI goes red", next: "ran 2h ago", isOn: true),
            PanelRoutine(name: "Weekly lessons", when: "Fridays 17:00", next: "", isOn: false),
        ],
        places: [
            PanelPlace(name: "Queue", symbol: "list.number", count: 6),
            PanelPlace(name: "Events", symbol: "bolt.horizontal.fill", count: 12),
        ],
        memory: 9
    )

    internal static let coder = PanelContent(
        screen: "localhost:4173/look",
        sites: 2,
        terminal: [
            "$ npx playwright test look",
            "  ✓ 31 passed",
            "  ✗ 1 failed  look sheet › scene",
            "  running 32/50…",
        ],
        routines: [
            PanelRoutine(name: "Nightly e2e", when: "Every day 02:00", next: "next tonight 02:00", isOn: true)
        ],
        memory: 4
    )

    internal static func fixture(_ personaID: String) -> PanelContent {
        switch personaID {
        case "cond": conductor
        case "coder": coder
        default: PanelContent()
        }
    }
}
