import CooeeCore

extension JSONValue {
    internal static let personaPanels = [
        "cond": JSONValue(
            json: #"""
                {"type": "screen", "id": "persona", "title": "Conductor", "blocks": [
                  {"type": "screen", "url": "github.com/jirathip-dev/cooee/pulls", "sites": 3},
                  {"type": "terminal", "lines": ["$ herdsman board cooee", "#244 design   live  2m",
                    "#236 queued   after #244", "#229 review1  PASS  ready", "#213 merged"]},
                  {"type": "workflow", "more": 4, "issues": [
                    {"number": 244, "title": "Persona panel design", "stage": "design", "tone": "working"},
                    {"number": 229, "title": "Workflow sheet", "stage": "ready · merge?", "tone": "needs"},
                    {"number": 217, "title": "Persona history tool", "stage": "merged", "tone": "done"}]},
                  {"type": "routines", "rows": [
                    {"name": "Morning board", "when": "Weekdays 08:30", "next": "next tomorrow 08:30", "on": true},
                    {"name": "Stale PR sweep", "when": "When CI goes red", "next": "ran 2h ago", "on": true},
                    {"name": "Weekly lessons", "when": "Fridays 17:00", "on": false}]},
                  {"type": "places", "rows": [{"title": "Queue", "symbol": "list.number", "value": "6"},
                    {"title": "Events", "symbol": "bolt.horizontal.fill", "value": "12"}]},
                  {"type": "memory", "count": 9}, {"type": "edit"}]}
                """#),
        "coder": JSONValue(
            json: #"""
                {"type": "screen", "id": "persona", "title": "Coder", "blocks": [
                  {"type": "screen", "url": "localhost:4173/look", "sites": 2},
                  {"type": "terminal", "lines": ["$ npx playwright test look", "  ✓ 31 passed",
                    "  ✗ 1 failed  look sheet › scene", "  running 32/50…"]},
                  {"type": "routines", "rows": [
                    {"name": "Nightly e2e", "when": "Every day 02:00", "next": "next tonight 02:00", "on": true}]},
                  {"type": "memory", "count": 4}, {"type": "edit"}]}
                """#),
    ]

    internal static let chiefEdit = JSONValue(
        json: #"""
            {"type": "screen", "id": "persona", "title": "Chief", "blocks": [{"type": "edit"},
              {"type": "memory", "count": 12, "notes": [
                {"text": "Account Manager knows the CRM; ask it for dates and owners", "by": "persona",
                 "at": 1799740800000},
                {"text": "Renewals are tracked in renewals.md, one line each", "by": "owner", "at": 1799913600000},
                {"text": "Dana Okafor runs Kestrel's finance team, since September", "by": "persona",
                 "at": 1799992800000}]}]}
            """#)

    internal static let chiefEditNow: Double = 1_800_000_000
}
