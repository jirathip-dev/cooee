import CooeeCore
import CooeeUI
import SwiftUI
import Testing

@Suite(.serialized)
internal struct RoomFrames {
    internal static var all: [Frame] {
        [
            .phone("room-dark", .farmDark) { RoomView.kestrel() },
            .phone("room-light", .farmLight) { RoomView.kestrel() },
            .phone("room-create-dark", .farmDark) { NewRoomSheet.kestrel() },
            .phone("room-create-light", .farmLight) { NewRoomSheet.kestrel() },
            .phone("room-member-dark", .farmDark) { MemberSheet.researcher() },
            .phone("room-home-dark", .farmDark) { HomeView(chats: Fixtures.roomChats) },
            .phone("room-pinned-dark", .farmDark) { HomeView(pins: Fixtures.roomPins, chats: Fixtures.roomChats) },
            .phone("room-mention-dark", .farmDark) { RoomView.mentioning() },
            .phone("room-ask-dark", .farmDark) { RoomView.asking() },
            .mac("room-wide-dark", 1440, 900, .farmDark) { Sidebar.kestrel() },
            .phone("asked-dark", .farmDark) { ChatView.asked() },
            .phone("asked-light", .farmLight) { ChatView.asked() },
            .phone("linked-dark", .farmDark) { LinkedView.researcher() },
            .mac("linked-wide-light", 1440, 900, .farmLight) { Sidebar.linked() },
        ]
    }

    @MainActor @Test(arguments: all)
    internal func matchesTheDesign(_ frame: Frame) throws { try frame.matchesItsDesign() }

    @MainActor @Test(arguments: all)
    internal func fitsInEveryPack(_ frame: Frame) throws { try frame.fitsInEveryPack() }
}
