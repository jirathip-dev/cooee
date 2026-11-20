/// A line linked to another thread (#553 row 35): a handoff's `continued in`, or a line `from` it.
public struct ThreadLink: Sendable, Equatable {
    public let text: String
    public let thread: String
    public let title: String
    public let onward: Bool

    public init(text: String = "", thread: String, title: String, onward: Bool) {
        self.text = text
        self.thread = thread
        self.title = title
        self.onward = onward
    }

    public init?(_ item: APIItem) {
        guard let thread = item.thread, !thread.isEmpty else { return nil }
        self.init(text: item.text ?? "", thread: thread, title: item.title ?? "", onward: item.out == true)
    }

    /// The chip's words: `continued in <thread>` or `from <thread>`.
    public var chip: String {
        let name = title.isEmpty ? thread : title
        return onward ? "continued in \(name)" : "from \(name)"
    }
}
