import CooeeFeatures
import SwiftUI

extension ChatScreen {
    internal func panel(column: Bool) -> PersonaPanel {
        let (persona, places, attachable) = (store.persona, store.places, store.attachable)
        let pick: (PanelPick) -> Void = { store.send(.panel(column ? .run($0) : .pick($0))) }
        let known = !persona.api.id.isEmpty
        let actions = PanelActions(
            close: { store.send(.panel(.show(false))) },
            edit: known ? { pick(.edit) } : nil,
            newChat: known ? { pick(.newChat) } : nil,
            stop: store.working ? { pick(.stop) } : nil,
            open: { pick(.open(places[$0])) },
            attach: { pick(.attach(attachable[$0])) }
        )
        let content = PanelContent(
            routines: nil,
            places: places.map { PanelPlace(name: $0.label, symbol: "square.grid.2x2", count: 0) },
            attachable: attachable.map(\.name)
        )
        return PersonaPanel(persona: persona, content: content, column: column, actions: actions)
    }
}
