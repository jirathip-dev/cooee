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
            state.answering.remove(item)
            return error.map { toast(&state, $0) } ?? .none
        }
    }

    private func answer(_ state: inout State, _ item: Int, _ action: BlockAction) -> Effect<Action> {
        guard case .thread(var thread) = state.target, let answer = action.answer else { return .none }
        var asked = item
        if let ask = state.room?.ask(item) { (thread, asked) = (ask.session, ask.item.id) }
        state.answering.insert(item)
        let secret = state.items.first { $0.id == item }?.block?.string("kind") == "secret"
        let (threads, server, owner) = (threads, state.server, ownerCheck)
        return .run { [thread, asked] send in
            if secret, await !owner.confirm("Send the secret to its plugin") {
                return await send(.cards(.answered(item, "Not sent: it wasn't confirmed it's you.")))
            }
            try await threads.answer(server, thread, asked, answer.option, answer.value)
            await send(.cards(.answered(item, nil)))
        } catch: { error, send in
            await send(.cards(.answered(item, error.localizedDescription)))
        }
    }
}
