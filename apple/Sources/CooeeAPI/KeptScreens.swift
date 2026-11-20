import CooeeCore
import Foundation

public struct KeptScreens: Sendable {
    private let folder: URL

    public init(folder: URL = URL.applicationSupportDirectory.appending(path: "screens", directoryHint: .isDirectory)) {
        self.folder = folder
    }

    private func file(_ key: String) -> URL {
        folder.appending(path: Data(key.utf8).base64EncodedString().replacingOccurrences(of: "/", with: "_"))
    }

    public func load<Kept: Decodable>(_ key: String) -> Kept? {
        guard let data = try? Data(contentsOf: file(key)) else { return nil }
        return try? JSONDecoder().decode(Kept.self, from: data)
    }

    public func save(_ key: String, _ kept: some Encodable) {
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        try? JSONEncoder().encode(kept).write(to: file(key), options: .atomic)
    }
}
