import ComposableArchitecture
import CooeeFeatures
import Foundation

internal actor DraftShelf {
    private let folder: URL
    private let pending = LockIsolated<[String: Draft]>([:])

    internal init(folder: URL) {
        self.folder = folder
    }

    nonisolated internal func hold(_ key: String, _ draft: Draft) {
        pending.withValue { $0[key] = draft }
    }

    nonisolated internal func load(_ key: String) -> Draft {
        if let held = pending.value[key] { return held }
        let data = (try? Data(contentsOf: file(key))) ?? Data()
        return (try? JSONDecoder().decode(Draft.self, from: data)) ?? Draft()
    }

    internal func flush(_ key: String) {
        guard let draft = pending.value[key] else { return }
        if draft.isEmpty {
            try? FileManager.default.removeItem(at: file(key))
        } else {
            try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            try? JSONEncoder().encode(draft).write(to: file(key), options: .atomic)
        }
        pending.withValue { if $0[key] == draft { $0[key] = nil } }
    }

    nonisolated private func file(_ key: String) -> URL {
        let name = Data(key.utf8).base64EncodedString().replacing("/", with: "_")
        return folder.appending(path: "\(name).json")
    }
}

extension DraftStore: DependencyKey {
    internal static func live(_ shelf: DraftShelf) -> DraftStore {
        DraftStore(
            load: { key in shelf.load(key) },
            save: { key, draft in
                shelf.hold(key, draft)
                Task { await shelf.flush(key) }
            }
        )
    }

    public static let liveValue = live(
        DraftShelf(folder: URL.applicationSupportDirectory.appending(path: "drafts", directoryHint: .isDirectory)))
}
