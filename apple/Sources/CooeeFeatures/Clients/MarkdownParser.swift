import ComposableArchitecture
import CooeeCore

private enum MarkdownParser: DependencyKey {
    fileprivate static let liveValue: @Sendable (String) -> [Block] = { Markdown.blocks($0) }
    fileprivate static let testValue = liveValue
}

extension DependencyValues {
    /// Splits a reply's markdown into blocks; a test swaps in a counting one.
    public var markdown: @Sendable (String) -> [Block] {
        get { self[MarkdownParser.self] }
        set { self[MarkdownParser.self] = newValue }
    }
}
