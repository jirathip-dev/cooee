@testable import CooeeApp
import CooeeCore
import CooeeFeatures
import Foundation
import Testing

internal struct DraftShelfTests {
    @Test internal func aDraftWithAFileReadsBackAtOnceAndFromDiskAfterItsWrite() async throws {
        let folder = FileManager.default.temporaryDirectory.appending(path: "drafts-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: folder) }
        let shelf = DraftShelf(folder: folder)
        let store = DraftStore.live(shelf)
        let draft = Draft(text: "a long draft", files: [Attachment(name: "a.png", bytes: Data(count: 1 << 20))])
        store.save("t1", draft)
        #expect(store.load("t1") == draft)
        await shelf.flush("t1")
        #expect(DraftStore.live(DraftShelf(folder: folder)).load("t1") == draft)
        store.save("t1", Draft())
        #expect(store.load("t1") == Draft())
        await shelf.flush("t1")
        #expect(DraftStore.live(DraftShelf(folder: folder)).load("t1") == Draft())
        #expect(try FileManager.default.contentsOfDirectory(atPath: folder.path()).isEmpty)
    }
}
