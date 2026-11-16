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
            open: { $0 < places.count ? pick(.open(places[$0])) : pick(.attach(attachable[$0 - places.count])) }
        )
        let rows =
            places.map { Self.row("square.grid.2x2", $0.label) }
            + attachable.map { Self.row("plus", "Attach \($0.name)…") }
        let screen = JSONValue.object(["blocks": .array([.object(["type": .string("places"), "rows": .array(rows)])])])
        return PersonaPanelBlocks(persona: persona, screen: screen, column: column, actions: actions)
    }

    private static func row(_ symbol: String, _ title: String) -> JSONValue {
        .object(["symbol": .string(symbol), "title": .string(title)])
    }
}
