import ComposableArchitecture
import CooeeCore
import Foundation

/// The composer's dictation: the draft it began from, what it heard and whether it still listens.
public struct Dictation: Sendable, Equatable {
    public var before: String
    public var heard = Heard()
    public var listening = true

    public var text: String { [before, heard.text].filter { !$0.isEmpty }.joined(separator: " ") }

    public init(before: String, heard: Heard = Heard(), listening: Bool = true) {
        self.before = before
        self.heard = heard
        self.listening = listening
    }
}

/// A call with the thread's persona: when it began, the last reply spoken and the call screen's toggles.
public struct Calling: Sendable, Equatable {
    public let started: Date
    public let from: Int
    public var spoken: Int
    public var speaking: String?
    public var muted = false
    public var speaker = false
    public var shown = true
}

extension ChatFeature {
    @CasePathable
    public enum Voice: Sendable, Equatable {
        case mic
        case heard(Heard)
        case pick(String)
        case hold(Bool)
        case noted(Attachment)
    }

    private enum Listening: Hashable { case dictation }

    internal func voice(_ state: inout State, _ action: Voice) -> Effect<Action> {
        @Dependency(\.voice) var voice
        switch action {
        case .mic where state.dictation?.listening == true:
            settle(&state)
            return .cancel(id: Listening.dictation)
        case .mic:
            state.dictation = Dictation(before: state.draft)
            return .run { send in
                for try await heard in voice.dictate() { await send(.voice(.heard(heard))) }
                await send(.voice(.mic))
            } catch: { error, send in
                await send(.failed(error.localizedDescription))
            }
            .cancellable(id: Listening.dictation, cancelInFlight: true)
        case .heard(let heard):
            state.dictation?.heard = heard
        case .pick(let word):
            pick(&state, word)
        case .hold(let down):
            return note(down)
        case .noted(let note):
            state.files.append(note)
            keep(state)
        }
        return .none
    }

    private func settle(_ state: inout State) {
        guard var dictation = state.dictation else { return }
        state.draft = dictation.text
        dictation.listening = false
        state.dictation = dictation.heard.unsure.isEmpty ? nil : dictation
        keep(state)
    }

    private func pick(_ state: inout State, _ word: String) {
        guard var dictation = state.dictation, let unsure = dictation.heard.unsure.first else { return }
        if let range = state.draft.range(of: unsure.word) { state.draft.replaceSubrange(range, with: word) }
        dictation.heard.unsure.removeFirst()
        state.dictation = dictation.heard.unsure.isEmpty ? nil : dictation
        keep(state)
    }

    private func note(_ down: Bool) -> Effect<Action> {
        @Dependency(\.voice) var voice
        return .run { send in
            if down { try await voice.record() } else { await send(.voice(.noted(try await voice.recorded()))) }
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}

extension ChatFeature.State {
    internal mutating func edited(_ text: String) {
        guard var dictation, !dictation.listening else { return }
        dictation.heard.unsure.removeAll { !text.contains($0.word) }
        self.dictation = dictation.heard.unsure.isEmpty ? nil : dictation
    }
}
