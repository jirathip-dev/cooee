import CooeeCore
@testable import CooeeUI
import SwiftUI
import Testing

@MainActor
internal struct PinnedRowTests {
    private static let styles: [String: Style] = ["modern": .bundled, "terminal": .terminal, "scene": .scene]

    private static func home(_ style: String, pins: [Pin], chats: [ListedChat]) throws -> Pixels {
        let frame = Frame.phone("home", .farmLight) { HomeView(pins: pins, chats: chats) }
        let look = try #require(styles[style])
        return try #require(frame.render(look).flatMap(Pixels.init))
    }

    private static func firstDifferingPoint(_ one: Pixels, _ other: Pixels) throws -> Int {
        let row = one.width * 4
        let differs: (Int) -> Bool = { line in
            (line * row..<(line + 1) * row).contains { abs(Int(one.bytes[$0]) - Int(other.bytes[$0])) > 64 }
        }
        let pixel: Int? = (0..<one.height).first(where: differs)
        return try #require(pixel) / 2
    }

    @Test(arguments: ["modern", "terminal", "scene"])
    internal func withNothingPinnedTheChatsSitUnderTheTopBar(_ style: String) throws {
        let unpinned = try Self.home(style, pins: [], chats: Fixtures.chats)
        let pinned = try Self.home(style, pins: Fixtures.personas.map(Pin.persona), chats: Fixtures.chats)
        let empty = try Self.home(style, pins: [], chats: [])
        let pinnedTop = try Self.firstDifferingPoint(pinned, unpinned)
        let chatsTop = try Self.firstDifferingPoint(unpinned, empty)
        #expect(chatsTop - pinnedTop < 48, "\(style): the chats start \(chatsTop - pinnedTop) pt under the pinned row")
    }
}
