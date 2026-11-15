public struct PairAsk: Codable, Sendable, Equatable {
    public let word: String
    public let claim: String

    public init(word: String, claim: String) {
        self.word = word
        self.claim = claim
    }
}
