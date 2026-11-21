import Foundation

/// How a screen's link to its server stands, as its sync dot draws it (#369).
public enum Sync: Sendable, Equatable {
    case live
    case reconnecting
    case offline
}

/// A thread's newest slice as this device last saw it, drawn when the server can't be reached (#369).
public struct KeptSlice: Codable, Sendable, Equatable {
    public let slice: Slice
    public let kept: Date

    public init(slice: Slice, kept: Date) {
        self.slice = slice
        self.kept = kept
    }
}

/// Home's list as this device last saw it, drawn when the server can't be reached (#369).
public struct KeptHome: Codable, Sendable, Equatable {
    public let screen: HomeScreen
    public let threads: [ThreadRow]
    public let personas: [APIPersona]
    public let counts: [String: TopicCount]
    public let plugins: [PluginRow]
    public let kept: Date

    public init(_ answer: HomeAnswer, plugins: [PluginRow], kept: Date) {
        screen = answer.screen
        threads = answer.threads
        personas = answer.personas
        counts = answer.counts
        self.plugins = plugins
        self.kept = kept
    }
}
