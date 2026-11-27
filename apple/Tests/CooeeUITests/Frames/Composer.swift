import ComposableArchitecture
import CooeeCore
import CooeeFeatures
@testable import CooeeUI
import SwiftUI
import Testing

@MainActor
private enum ComposerScreens {
    internal static func bar(_ draft: String, width: CGFloat, left: CGFloat, bottom: CGFloat) -> some View {
        ZStack(alignment: .bottomLeading) {
            Backdrop()
            Composer(hint: "Message Coder", draft: draft).frame(width: width)
                .padding(.leading, left).padding(.bottom, bottom)
        }
    }
}

@Suite(.serialized)
internal struct ComposerFrames {
    private static let modes: [(String, Palette)] = [("dark", .farmDark), ("light", .farmLight)]
    private static let styles: [(String, Style)] = [("modern", .bundled), ("terminal", .terminal), ("scene", .scene)]
    private static let typed = "Rerun it 50 times on CI\nand post the flake rate"
    private static let phoneBar = CGRect(x: 0, y: 515, width: Frame.phoneWidth, height: 66)
    private static let macBar = CGRect(x: 380, y: 730, width: 880, height: 52)

    private static func phone(_ name: String, _ palette: Palette, _ draft: String) -> Frame {
        Frame.phone(name, palette) {
            ComposerScreens.bar(draft, width: Frame.phoneWidth - 24, left: 12, bottom: Frame.phoneHeight - 577)
        }
    }

    internal static let all: [(Style, Frame)] =
        modes.flatMap { mode, palette in
            [
                (Style.bundled, phone("bar-modern-\(mode)", palette, "")),
                (.bundled, phone("typing-modern-\(mode)", palette, typed)),
                (.scene, phone("typing-scene-\(mode)", palette, typed)),
            ]
                + styles.map { name, style in
                    let frame = Frame.mac("mac-\(name)-\(mode)", 1280, 800, palette) {
                        ComposerScreens.bar("/m", width: 760, left: 440, bottom: 20)
                    }
                    return (style, frame)
                }
        }

    @MainActor @Test(arguments: all.indices)
    internal func theBarMatchesTheDesign(_ index: Int) throws {
        let (style, frame) = Self.all[index]
        try frame.filed("composer-v2").matchesItsDesign(style, only: frame.phone ? Self.phoneBar : Self.macBar)
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

    @MainActor @Test internal func returnSendsWhereverItIsTypedAndShiftReturnAddsALine() throws {
        let coder = Fixtures.persona("coder")
        let server = Server(url: URL(filePath: "/mini"), token: "t")
        let store = Store(initialState: ChatFeature.State(server: server, target: .thread("a1"), persona: coder)) {
            ChatFeature()
        } withDependencies: {
            $0.threads.send = { _, _, _, _, _ in }
            $0.desk.warm = { _, _ in }
            $0.continuousClock = TestClock()
        }
        let actions = ChatScreen.actions(store) {}
        let draft = try #require(actions.draft)
        let typed = Composer.keyed(draft) { actions.send?() }
        typed.wrappedValue = "pasted\nlines"
        #expect(store.draft == "pasted\nlines")
        typed.wrappedValue = "Rerun it"
        Composer.pressed(shift: true, draft) { actions.send?() }
        typed.wrappedValue = "Rerun it\nand post"
        #expect(store.draft == "Rerun it\nand post")
        typed.wrappedValue = "Rerun it\nand\n post"
        #expect(store.draft.isEmpty)
        #expect(store.outbox.map(\.text) == ["Rerun it\nand post"])
        typed.wrappedValue = "Then"
        Composer.pressed(shift: false, draft) { actions.send?() }
        #expect(store.outbox.map(\.text).last == "Then")
    }
}
