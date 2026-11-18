import CooeeCore
import CooeeUI
import Foundation
import Testing

internal struct DefaultLayoutTests {
    private static func places(_ node: LayoutNode<LayoutPlace>) -> [String: LayoutPlace] {
        node.slots.values.joined().reduce(into: [node.id: node.place]) { $0.merge(places($1)) { old, _ in old } }
    }

    @Test internal func theDefaultHoldsBothLockedScreensLast() {
        let root = CooeeCore.Layout.bundled.root
        #expect(CooeeCore.Layout.bundled.etag.isEmpty && root.content == .component(.nav))
        #expect(root.slots["main"]?.map(\.id) == ["home", "chat", "pairing"])
        #expect(root.slots["menu"]?.map(\.id) == ["menu", "settings", "terminal", "reset-layout"])
        #expect(root.slots["main"]?.last?.locked == true && root.slots["menu"]?.last?.locked == true)
        #expect(root.slots.values.joined().filter(\.locked).count == 2)
    }

    @Test internal func theDefaultResolvesTodaysPlacesOnEachSize() {
        #expect(
            Self.places(CooeeCore.Layout.bundled.resolved(for: .compact)) == [
                "nav": .inline, "sidebar": .menu, "home": .push, "chat": .push, "pairing": .push,
                "menu": .sheet, "settings": .sheet, "terminal": .menu, "reset-layout": .menu,
            ])
        #expect(
            Self.places(CooeeCore.Layout.bundled.resolved(for: .wide)) == [
                "nav": .inline, "sidebar": .column, "home": .panel, "chat": .push, "pairing": .push,
                "menu": .sheet, "settings": .sheet, "terminal": .menu, "reset-layout": .menu,
            ])
    }

    @Test internal func theBundledHomeDrawsItsPinnedRowThenItsChats() {
        #expect(HomeScreen.bundled.title == "cooee")
        #expect(HomeScreen.bundled.blocks.map(\.type) == ["pinned", "chats"])
    }

    @Test internal func everyBundledStyleDecodesToItsTokens() {
        let styles = LookStyle.allCases.map(Style.bundled)
        #expect(styles.map(\.label) == ["Modern", "Terminal", "Scene"])
        #expect(styles.map(\.surface) == [.glass, .dashed, .solid])
        #expect(styles.map(\.figures) == [.avatar, .ascii, .sprite])
        #expect(styles.map(\.type) == [.default, .mono, .default])
        #expect(styles.map(\.icon) == [.modern, .terminal, .farm])
        #expect(styles.map { $0.stage?.alertPaper } == [nil, nil, "#f8dede"])
    }

    @Test(arguments: ["terminal", "scene"])
    internal func theBundledPackStyleIsThePacksFileByteForByte(_ pack: String) throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent()
        let bundled = root.appendingPathComponent("Sources/CooeeUI/Default/styles/\(pack).json")
        let packed = root.deletingLastPathComponent().appendingPathComponent("packs/\(pack)/style.json")
        #expect(try Data(contentsOf: bundled) == Data(contentsOf: packed))
    }

    private static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .appendingPathComponent("packs/fixtures/styles")

    private static func style(_ name: String) throws -> Style {
        try JSONDecoder().decode(Style.self, from: Data(contentsOf: fixtures.appendingPathComponent(name)))
    }

    @Test(arguments: ["bad-prefix.json", "bad-suffix.json"])
    internal func aStyleWhoseLabelAddsWordsIsRefused(_ name: String) {
        #expect(throws: DecodingError.self) { try Self.style(name) }
    }

    @Test internal func everyLabelOfTheWidestStyleKeepsItsWholeText() throws {
        let worst = try Self.style("ok-worst-case.json")
        let text = "Secret · from Kestrel"
        #expect(worst.labels.count == 10)
        for label in worst.labels.values {
            #expect(label.apply(text).lowercased().contains(text.lowercased()))
        }
        #expect(worst.labels[.kind]?.apply(text) == "[## SECRET · FROM KESTREL ##]")
        #expect(try Self.style("ok-modern.json") == Style.bundled(.modern))
    }
}
