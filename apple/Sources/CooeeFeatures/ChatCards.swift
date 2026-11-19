import ComposableArchitecture
import CooeeCore

extension ChatFeature {
    @CasePathable
    public enum Cards: Sendable, Equatable {
        case answer(Int, BlockAction)
        case answered(Int, String?)
        case heard(String, JSONValue)
        case played(Int, JSONValue, String?)
    }

    internal func cards(_ state: inout State, _ action: Cards) -> Effect<Action> {
        switch action {
        case .answer(let item, let action) where state.deck.holds(item):
            return play(&state, item, action)
        case .answer(let item, let action):
            return answer(&state, item, action)
        case .heard(let plugin, let event):
            state.deck.heard(plugin, event)
            return .none
        case .played(let item, let reply, let line):
            state.answering.remove(item)
            state.deck.played(item, reply, line: line)
            return .none
        case .answered(let item, let error):
            if error != nil || !state.awaitsLine(item) { state.answering.remove(item) }
            return error.map { toast(&state, $0) } ?? .none
        }
    }

    internal func answer(_ state: inout State, _ item: Int, _ action: BlockAction) -> Effect<Action> {
        guard case .thread(var thread) = state.target, let answer = action.answer else { return .none }
        if answer.option == "say", answer.value == nil {
            state.denying = item
            return .none
        }
        var asked = item
        if let ask = state.room?.ask(item) { (thread, asked) = (ask.session, ask.item.id) }
        state.answering.insert(item)
        let card = state.items.first { $0.id == item }
        let kind = card?.block?.string("kind")
        let proved = answer.option == "approve" && card?.plugin == nil && ["proposal", "secret"].contains(kind)
        let (threads, server, owner) = (threads, state.server, ownerCheck)
        return .run { [thread, asked] send in
            if proved {
                try await threads.approve(server, thread, asked, answer.value)
            } else if kind == "secret", await !owner.confirm("Send the secret to its plugin") {
                return await send(.cards(.answered(item, "Not sent: it wasn't confirmed it's you.")))
            } else {
                try await threads.answer(server, thread, asked, answer.option, answer.value)
            }
            await send(.cards(.answered(item, nil)))
        } catch: { error, send in
            await send(.cards(.answered(item, error.localizedDescription)))
        }
    }
}

extension ChatFeature.State {
    internal func awaitsLine(_ id: Int) -> Bool {
        items.contains { $0.id == id && $0.line == nil && $0.block?.string("kind") == "config" }
    }

    internal func applying(_ item: Item) -> Item {
        guard case .card(let id, let poster, let block) = item, answering.contains(id), awaitsLine(id),
            case .object(var card) = block
        else { return item }
        let note: JSONValue = .object(
            ["type": .string("text"), "style": .string("footnote"), "markdown": .string("applying…")])
        card["blocks"] = .array(Array(block.list("blocks").prefix(1)) + [note])
        return .card(id: id, poster: poster, block: .object(card))
    }
}
