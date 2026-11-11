import CooeeCore
import CooeeUI
import SwiftUI
import Testing

@MainActor
internal struct PinnedRowTests {
    private static func home(_ style: LookStyle, pinned: [Persona], chats: [ListedChat]) throws -> Pixels {
        let frame = Frame(
            name: "home", width: 402, height: 874, palette: .farmLight, style: style, phone: true,
            view: { AnyView(HomeView(personas: pinned, all: Fixtures.personas, chats: chats)) }
        )
        return try #require(frame.render().flatMap(Pixels.init))
    }

    private static func firstDifferingPoint(_ one: Pixels, _ other: Pixels) throws -> Int {
        let row = one.width * 4
        let pixel = try #require(
            (0..<one.height).first { one.bytes[$0 * row..<($0 + 1) * row] != other.bytes[$0 * row..<($0 + 1) * row] })
        return pixel / 2
    }

    @Test(arguments: LookStyle.allCases)
    internal func withNothingPinnedTheChatsSitUnderTheTopBar(_ style: LookStyle) throws {
        let unpinned = try Self.home(style, pinned: [], chats: Fixtures.chats)
        let pinnedTop = try Self.firstDifferingPoint(
            try Self.home(style, pinned: Fixtures.personas, chats: Fixtures.chats), unpinned)
        let chatsTop = try Self.firstDifferingPoint(unpinned, try Self.home(style, pinned: [], chats: []))
        #expect(chatsTop - pinnedTop < 48, "\(style): the chats start \(chatsTop - pinnedTop) pt under the pinned row")
    }
}
