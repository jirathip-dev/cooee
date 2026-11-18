import ComposableArchitecture
import CooeeCore
import Foundation

/// The Screen sheet (#166): the persona's browser live, the lease while the owner holds it, a replayed frame.
public struct BrowserSheet: Equatable, Sendable {
    public var screen = BrowserScreen()
    public var live: LiveFrame?
    public var gone: String?
    public var replay: String?
    public var now: Date
    public var note = ""
    public var typed = ""
    public var error: String?

    public var left: Int? { screen.left(at: now) }
    public var held: Bool { left != nil }
}

extension ChatFeature {
    @CasePathable
    public enum Browse: Sendable, Equatable {
        case show(Bool)
        case open(String)
        case takeOver
        case done
        case watch
        case screen(BrowserScreen)
        case live(LiveEvent)
        case tick
        case take
        case handBack
        case note(String)
        case typed(String)
        case send(BrowserInput)
        case replay(String?)
        case failed(String)
    }

    internal func browser(_ state: inout State, _ action: Action) -> Effect<Action> {
        guard case .browse(let action) = action else { return .none }
        let (threads, server, persona) = (threads, state.server, state.persona.api.id)
        switch action {
        case .show(let open):
            state.browser = open ? BrowserSheet(now: now) : nil
        case .open(let frame):
            state.browser = BrowserSheet(replay: frame, now: now)
        case .takeOver:
            state.browser = BrowserSheet(now: now)
            return lease { try await threads.takeControl(server, persona) }
        case .watch:
            return watch(server, persona)
        default:
            return drive(&state, action)
        }
        return .none
    }

    private func drive(_ state: inout State, _ action: Browse) -> Effect<Action> {
        let (threads, server, persona) = (threads, state.server, state.persona.api.id)
        switch action {
        case .take:
            return lease { try await threads.takeControl(server, persona) }
        case .handBack:
            let note = state.browser?.note ?? ""
            state.browser?.note = ""
            return .merge(lease { try await threads.handBack(server, persona, note) }, resume(state, note))
        case .done:
            return resume(state, "")
        case .send(let input):
            return type(&state, input)
        default:
            if state.browser != nil { apply(&state, action) }
            return .none
        }
    }

    private func resume(_ state: State, _ note: String) -> Effect<Action> {
        guard state.handoff != nil else { return .none }
        return act(state) { threads, server, thread in try await threads.resume(server, thread, note) }
    }

    private func apply(_ state: inout State, _ action: Browse) {
        switch action {
        case .screen(let screen):
            state.browser?.screen = screen
            state.browser?.error = nil
        case .live(.frame(let frame)):
            state.browser?.live = frame
            state.browser?.gone = nil
        case .live(.gone(let why)):
            state.browser?.gone = why
        case .tick:
            state.browser?.now = now
            if state.browser?.held == false { state.browser?.screen.until = nil }
        default:
            edit(&state, action)
        }
    }

    private func edit(_ state: inout State, _ action: Browse) {
        switch action {
        case .note(let text): state.browser?.note = text
        case .typed(let text): state.browser?.typed = text
        case .replay(let name): state.browser?.replay = name
        case .failed(let error): state.browser?.error = error
        default: break
        }
    }

    private func type(_ state: inout State, _ input: BrowserInput) -> Effect<Action> {
        let (threads, server, persona) = (threads, state.server, state.persona.api.id)
        if case .text = input { state.browser?.typed = "" }
        return .run { _ in
            try await threads.input(server, persona, input)
        } catch: { error, send in
            await send(.browse(.failed(error.localizedDescription)))
        }
    }

    private func lease(_ call: @escaping @Sendable () async throws -> BrowserScreen) -> Effect<Action> {
        .run { send in
            await send(.browse(.screen(try await call())))
        } catch: { error, send in
            await send(.browse(.failed(error.localizedDescription)))
        }
    }

    private func watch(_ server: Server, _ persona: String) -> Effect<Action> {
        let (threads, clock) = (threads, clock)
        return .merge(
            lease { try await threads.screen(server, persona) },
            .run { send in
                for try await event in threads.live(server, persona) { await send(.browse(.live(event))) }
            } catch: { error, send in
                await send(.browse(.failed(error.localizedDescription)))
            },
            .run { send in
                while true {
                    try await clock.sleep(for: .seconds(1))
                    await send(.browse(.tick))
                }
            }
        )
    }
}
