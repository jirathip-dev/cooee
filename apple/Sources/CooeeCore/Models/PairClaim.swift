public enum PairClaim: Sendable, Equatable {
    case paired(String)
    case waiting
    case declined(String)
    case gone(String)
}
