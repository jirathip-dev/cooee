import ComposableArchitecture
import CooeeCore

/// A thread's unsent text and picked files, kept on the device until it is sent.
public struct Draft: Codable, Sendable, Equatable {
    public var text = ""
    public var files: [Attachment] = []

    public var isEmpty: Bool { text.isEmpty && files.isEmpty }

    public init(text: String = "", files: [Attachment] = []) {
        self.text = text
        self.files = files
    }
}

public struct DraftStore: Sendable {
    public var load: @Sendable (_ key: String) -> Draft
    public var save: @Sendable (_ key: String, _ draft: Draft) -> Void

    public init(
        load: @escaping @Sendable (_ key: String) -> Draft,
        save: @escaping @Sendable (_ key: String, _ draft: Draft) -> Void
    ) {
        self.load = load
        self.save = save
    }
}

extension DraftStore: TestDependencyKey {
    public static let testValue = DraftStore(load: { _ in Draft() }, save: { _, _ in })
}

extension DependencyValues {
    public var drafts: DraftStore {
        get { self[DraftStore.self] }
        set { self[DraftStore.self] = newValue }
    }
}
