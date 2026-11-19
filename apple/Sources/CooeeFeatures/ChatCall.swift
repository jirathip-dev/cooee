import ComposableArchitecture
import CooeeCore
import Foundation

extension ChatFeature {
    @CasePathable
    public enum CallAction: Sendable, Equatable {
        case start
        case said(String)
        case spoken
        case mute
        case speaker
        case shown(Bool)
        case end
    }

    private enum Calls: Hashable { case hearing, speaking }

    internal func call(_ state: inout State, _ action: CallAction) -> Effect<Action> {
        @Dependency(\.voice) var voice
        switch action {
        case .start where state.call != nil:
            state.call?.shown = true
        case .start:
            guard case .thread = state.target else { return .none }
            let last = state.items.map(\.id).max() ?? 0
            state.call = Calling(started: now, from: last, spoken: last)
            return hear()
        case .said(let text):
            return act(state) { [queue = state.working] threads, server, thread in
                try await threads.say(server, thread, text, queue)
            }
        case .spoken:
            state.call?.speaking = nil
        case .mute, .speaker, .shown:
            return toggle(&state, action)
        case .end:
            state.call = nil
            return .merge(.cancel(id: Calls.hearing), .cancel(id: Calls.speaking), .run { _ in await voice.hush() })
        }
        return .none
    }

    private func toggle(_ state: inout State, _ action: CallAction) -> Effect<Action> {
        switch action {
        case .mute:
            state.call?.muted.toggle()
            return state.call?.muted == true ? .cancel(id: Calls.hearing) : hear()
        case .speaker:
            state.call?.speaker.toggle()
            @Dependency(\.voice) var voice
            return .run { [loud = state.call?.speaker == true] _ in await voice.speaker(loud) }
        case .shown(let shown):
            state.call?.shown = shown
        default:
            break
        }
        return .none
    }

    internal func spoken(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .voice(let action): voice(&state, action)
        case .call(let action): call(&state, action)
        default: reply(&state, action)
        }
    }

    private func reply(_ state: inout State, _ action: Action) -> Effect<Action> {
        guard case .event(.item(let item)) = action, item.kind == "agent", let call = state.call,
            item.id > call.spoken
        else { return .none }
        let text = String(Markdown.inline(item.text ?? "").characters)
        state.call?.spoken = item.id
        state.call?.speaking = text
        @Dependency(\.voice) var voice
        return .run { send in
            await voice.speak(text)
            await send(.call(.spoken))
        }
        .cancellable(id: Calls.speaking)
    }

    private func hear() -> Effect<Action> {
        @Dependency(\.voice) var voice
        return .run { send in
            for try await line in voice.hear() { await send(.call(.said(line))) }
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
        .cancellable(id: Calls.hearing, cancelInFlight: true)
    }
}
