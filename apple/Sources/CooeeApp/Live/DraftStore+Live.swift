import ComposableArchitecture
import CooeeFeatures
import Foundation

extension DraftStore: DependencyKey {
    private static let folder = URL.applicationSupportDirectory.appending(path: "drafts", directoryHint: .isDirectory)

    private static func file(_ key: String) -> URL {
        let name = Data(key.utf8).base64EncodedString().replacing("/", with: "_")
        return folder.appending(path: "\(name).json")
    }

    public static let liveValue = DraftStore(
        load: { key in
            let data = (try? Data(contentsOf: file(key))) ?? Data()
            return (try? JSONDecoder().decode(Draft.self, from: data)) ?? Draft()
        },
        save: { key, draft in
            guard !draft.isEmpty else {
                try? FileManager.default.removeItem(at: file(key))
                return
            }
            try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            try? JSONEncoder().encode(draft).write(to: file(key), options: .atomic)
        }
    )
}
