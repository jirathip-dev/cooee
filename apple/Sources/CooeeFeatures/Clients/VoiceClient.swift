import ComposableArchitecture
import CooeeCore

/// A word dictation was unsure of, with what else it may have been.
public struct Unsure: Sendable, Equatable {
    public let word: String
    public let alternatives: [String]

    public init(word: String, alternatives: [String]) {
        self.word = word
        self.alternatives = alternatives
    }

    /// The words of `text` that `alternatives` (other hearings of it) put differently, with up to three each.
    public static func between(_ text: String, _ alternatives: [String]) -> [Unsure] {
        let words = text.split(separator: " ")
        var found: [Unsure] = []
        for alternative in alternatives {
            let other = alternative.split(separator: " ")
            let head = zip(words, other).prefix { $0 == $1 }.count
            let most = min(words.count, other.count) - head
            let tail = min(zip(words.reversed(), other.reversed()).prefix { $0 == $1 }.count, most)
            let mine = words[head..<words.count - tail].joined(separator: " ")
            let theirs = other[head..<other.count - tail].joined(separator: " ")
            guard !mine.isEmpty, !theirs.isEmpty else { continue }
            let known = found.firstIndex { $0.word == mine }
            let had = known.map { found[$0].alternatives } ?? []
            let unsure = Unsure(word: mine, alternatives: Array((had + [theirs]).uniqued().prefix(3)))
            if let known { found[known] = unsure } else { found.append(unsure) }
        }
        return found
    }
}

extension Array where Element: Hashable {
    fileprivate func uniqued() -> [Element] {
        var seen: Set<Element> = []
        return filter { seen.insert($0).inserted }
    }
}

/// What dictation has heard so far: the settled words, the words still forming and the unsure ones.
public struct Heard: Sendable, Equatable {
    public var settled = ""
    public var pending = ""
    public var unsure: [Unsure] = []

    public var text: String { [settled, pending].filter { !$0.isEmpty }.joined(separator: " ") }

    public init(settled: String = "", pending: String = "", unsure: [Unsure] = []) {
        self.settled = settled
        self.pending = pending
        self.unsure = unsure
    }
}

/// On-device speech: dictation, a call's utterances, voice notes and the system voice for replies.
@DependencyClient
public struct VoiceClient: Sendable {
    public var dictate: @Sendable () -> AsyncThrowingStream<Heard, any Error> = { .finished() }
    public var hear: @Sendable () -> AsyncThrowingStream<String, any Error> = { .finished() }
    public var record: @Sendable () async throws -> Void
    public var recorded: @Sendable () async throws -> Attachment
    public var speak: @Sendable (_ text: String) async -> Void
    public var hush: @Sendable () async -> Void
    public var speaker: @Sendable (_ on: Bool) async -> Void
    public var calls: @Sendable () async -> AsyncStream<String> = { .finished }
}

extension VoiceClient: TestDependencyKey {
    public static let testValue = VoiceClient()
}

extension DependencyValues {
    public var voice: VoiceClient {
        get { self[VoiceClient.self] }
        set { self[VoiceClient.self] = newValue }
    }
}
