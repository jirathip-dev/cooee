import ComposableArchitecture
import CooeeApp
import CooeeFeatures
import Testing

@MainActor
internal struct CooeeRootTests {
    private func store(_ root: CooeeRoot) -> StoreOf<AppFeature>? {
        Mirror(reflecting: root).descendant("store") as? StoreOf<AppFeature>
    }

    @Test internal func rebuildingTheRootKeepsItsStore() throws {
        let first = try #require(store(CooeeRoot()))
        let rebuilt = try #require(store(CooeeRoot()))
        #expect(first === rebuilt)
    }
}
