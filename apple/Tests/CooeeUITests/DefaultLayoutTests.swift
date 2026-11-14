import CooeeCore
import CooeeUI
import Testing

internal struct DefaultLayoutTests {
    private static func places(_ node: LayoutNode<LayoutPlace>) -> [String: LayoutPlace] {
        node.slots.values.joined().reduce(into: [node.id: node.place]) { $0.merge(places($1)) { old, _ in old } }
    }

    @Test internal func theDefaultHoldsBothLockedScreensLast() {
        let root = CooeeCore.Layout.bundled.root
        #expect(CooeeCore.Layout.bundled.etag.isEmpty && root.content == .component(.nav))
        #expect(root.slots["main"]?.map(\.id) == ["home", "chat", "pairing"])
        #expect(root.slots["menu"]?.map(\.id) == ["menu", "settings", "reset-layout"])
        #expect(root.slots["main"]?.last?.locked == true && root.slots["menu"]?.last?.locked == true)
        #expect(root.slots.values.joined().filter(\.locked).count == 2)
    }

    @Test internal func theDefaultResolvesTodaysPlacesOnEachSize() {
        #expect(
            Self.places(CooeeCore.Layout.bundled.resolved(for: .compact)) == [
                "nav": .inline, "sidebar": .menu, "home": .push, "chat": .push, "pairing": .push,
                "menu": .sheet, "settings": .sheet, "reset-layout": .menu,
            ])
        #expect(
            Self.places(CooeeCore.Layout.bundled.resolved(for: .wide)) == [
                "nav": .inline, "sidebar": .column, "home": .panel, "chat": .push, "pairing": .push,
                "menu": .sheet, "settings": .sheet, "reset-layout": .menu,
            ])
    }
}
