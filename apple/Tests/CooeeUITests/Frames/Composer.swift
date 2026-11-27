import ComposableArchitecture
import CooeeCore
import CooeeFeatures
@testable import CooeeUI
import SwiftUI
import Testing

@MainActor
private enum ComposerScreens {
    private static let items: [Item] = [
        .you(id: 1, text: "The look sheet e2e failed twice on CI. Can you find why?"),
        .tools(
            id: 2,
            rows: [ToolRow(.search, "rg settled web/e2e", "ok"), ToolRow(.run, "cargo test -p cooee-core", "ok")]),
        .agent(
            id: 3,
            markdown: [
                .text(
                    "A race: the sheet slides in while the test taps **Save**. It now waits on `sheet.settled`, and passed 20 runs in a row."
                )
            ]
        ),
    ]

    internal static let typed = "Rerun it 50 times on CI\nand post the flake rate"

    internal static func phone(_ draft: String) -> some View {
        ChatView(personaID: "coder", items: items, draft: draft)
    }

    internal static func mac() -> some View {
        Sidebar(chats: Fixtures.chats, selected: "t1", actions: HomeActions()) {
            EmptyView()
        } detail: {
            ChatView(personaID: "coder", items: items, back: false)
        }
    }
}

@Suite(.serialized)
internal struct ComposerFrames {
    private static let modes: [(String, Palette)] = [("dark", .farmDark), ("light", .farmLight)]
    private static let styles: [(String, Style)] = [("modern", .bundled), ("terminal", .terminal), ("scene", .scene)]

    internal static let all: [(Style, Frame)] =
        modes.flatMap { mode, palette in
            [
                (Style.bundled, Frame.phone("bar-modern-\(mode)", palette) { ComposerScreens.phone("") }),
                (
                    .bundled,
                    Frame.phone("typing-modern-\(mode)", palette) { ComposerScreens.phone(ComposerScreens.typed) }
                ),
                (.scene, Frame.phone("typing-scene-\(mode)", palette) { ComposerScreens.phone(ComposerScreens.typed) }),
            ]
                + styles.map { name, style in
                    (style, Frame.mac("mac-\(name)-\(mode)", 1280, 800, palette) { ComposerScreens.mac() })
                }
        }

    @MainActor @Test(arguments: all.indices)
    internal func matchesTheDesign(_ index: Int) throws {
        let (style, frame) = Self.all[index]
        try frame.filed("composer-v2").matchesItsDesign(style)
    }

    @MainActor @Test internal func theRightButtonIsTheMicUntilThereIsTextAndAgainOnceItIsCleared() throws {
        let palette = Palette.farmDark
        let coder = Fixtures.persona("coder")
        let server = Server(url: URL(filePath: "/mini"), token: "t")
        let store = Store(initialState: ChatFeature.State(server: server, target: .new(coder.api), persona: coder)) {
            ChatFeature()
        }
        func right() throws -> ArraySlice<UInt8> {
            let composer = Composer(hint: "Message Coder", actions: ChatScreen.actions(store) {})
            let framed = composer.padding(12).frame(width: 402).background(palette.background)
                .environment(\.look, Look(palette: palette, still: true))
            let image = try #require(ImageRenderer(content: framed).cgImage)
            let pixels = try #require(Pixels(image))
            let start = (pixels.height / 2 * pixels.width + pixels.width - 12 - 6) * 4
            return pixels.bytes[start..<start + 4]
        }
        let swatch = palette.link.frame(width: 2, height: 2)
        let image = try #require(ImageRenderer(content: swatch).cgImage)
        let link = try #require(Pixels(image)).bytes[0..<4]
        #expect(try right() != link)
        store.send(.compose(.draftChanged("Rerun it")))
        #expect(try right() == link)
        store.send(.compose(.draftChanged("")))
        #expect(try right() != link)
    }
}
