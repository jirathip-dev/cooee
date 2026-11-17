import CooeeCore
import CooeeFeatures

extension ChatScreen {
    internal func panel(column: Bool) -> PersonaPanelBlocks {
        let (persona, places, attachable) = (store.persona, store.places, store.attachable)
        let pick: (PanelPick) -> Void = { store.send(.panel(column ? .run($0) : .pick($0))) }
        let known = !persona.api.id.isEmpty
        let actions = PanelActions(
            close: { store.send(.panel(.show(false))) },
            edit: known ? { pick(.edit) } : nil,
            newChat: known ? { pick(.newChat) } : nil,
            stop: store.working ? { pick(.stop) } : nil,
            open: { $0 < places.count ? pick(.open(places[$0])) : pick(.attach(attachable[$0 - places.count])) },
            workflow: { pick(.workflow) },
            routine: { pick(.routine) },
            teach: { pick(.teach($0)) }
        )
        let rows =
            places.map { Self.row("square.grid.2x2", $0.label, $0.count.map(String.init) ?? "") }
            + attachable.map { Self.row("plus", "Attach \($0.name)…") }
        let blocks = [
            Self.workflow(store.workflow ?? []), .object(["type": .string("routines"), "rows": .array([])]),
            .object(["type": .string("places"), "rows": .array(rows)]),
            .object(["type": .string("actions"), "rows": .array(store.teach)]),
        ]
        let screen = JSONValue.object(["blocks": .array(blocks)])
        return PersonaPanelBlocks(persona: persona, screen: screen, column: column, actions: actions)
    }

    private static func row(_ symbol: String, _ title: String, _ value: String = "") -> JSONValue {
        .object(["symbol": .string(symbol), "title": .string(title), "value": .string(value)])
    }

    private static func workflow(_ chats: [ListedChat]) -> JSONValue {
        let issues = Workflow.issues(chats)
        let rows = issues.prefix(3).map { issue -> JSONValue in
            let (row, label) = (issue.latest.row, issue.latest.workflow)
            return .object([
                "number": .number(Double(Int(label?.item.filter(\.isNumber) ?? "") ?? 0)),
                "title": .string(row.title), "stage": .string(label?.stage ?? ""),
                "tone": .string(row.state == .merged ? "done" : row.state.rawValue),
            ])
        }
        return .object([
            "type": .string("workflow"), "map": .bool(false), "issues": .array(Array(rows)),
            "more": .number(Double(issues.count - rows.count)),
        ])
    }
}
