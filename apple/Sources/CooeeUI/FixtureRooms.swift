import CooeeCore
import SwiftUI

extension Fixtures {
    public static let kestrel = ["chief", "coder", "res", "am"]

    public static var round: RoomRound {
        var round = RoomRound()
        round.asked(kestrel, named: [], when: .distantPast)
        round.began("chief")
        round.replied("chief")
        round.began("coder")
        round.began("res")
        round.replied("res")
        round.began("am")
        return round
    }

    public static let roomItems: [Item] = [
        .divider(id: 0, text: "Today 09:40"),
        .you(id: 1, text: "Kestrel signs on the 28th. What's still open before then?"),
        .said(
            id: 2, speaker: "chief", time: "09:40",
            markdown: [
                .text(
                    Markdown.inline(
                        "Three things: pricing sign-off, the data export and their Q3 numbers. The export shipped Thursday. @Researcher, can you confirm Q3?"
                    ))
            ]
        ),
        .said(
            id: 3, speaker: "res", time: "09:41",
            markdown: [.text(Markdown.inline("Q3 revenue was **$4.1M**, up 12% on Q2 (10-Q filed 6 Nov)."))]),
        .tools(
            id: 4,
            rows: [
                ToolRow(.search, "sec.gov", "done"), ToolRow(.read, "10-Q", "done"), ToolRow(.fetch, "sec.gov", "done"),
            ]
        ),
        .said(
            id: 5, speaker: "am", time: "now",
            markdown: [
                .text(
                    Markdown.inline(
                        "Pricing: a 3-year term at the Birch & Co rate, 8% off list. I'll draft the order form ▍"))
            ]
        ),
    ]

    public static var room: RoomScene {
        var scene = RoomScene(
            title: "Kestrel launch", members: kestrel.map(persona), others: [persona("cond")], round: round,
            items: roomItems, inspecting: nil
        )
        scene.elapsed = 134
        return scene
    }

    public static var roomPins: [Pin] {
        personas.prefix(2).map(Pin.persona) + roomChats.prefix(1).map(Pin.room)
    }

    public static var roomChats: [ListedChat] {
        let room = Row(
            id: "r1", persona: "chief", title: "Kestrel launch", last: "Account Manager: Pricing: a 3-year term…",
            ago: "now", unseen: 1, state: .working
        )
        let deck = Row(
            id: "r2", persona: "chief", title: "Board deck", last: "Chief: Slides 4–9 are in deck/q3.key.", ago: "2d",
            unseen: 0, state: .done
        )
        return [ListedChat(row: room, persona: persona("chief"), members: kestrel.map(persona))] + exchangeChats
            + [ListedChat(row: deck, persona: persona("chief"), members: ["chief", "am"].map(persona))]
            + chats.filter { ["t2", "t4"].contains($0.id) }
    }
}

extension RoomView {
    public static func kestrel(wide: Bool = false) -> RoomView {
        RoomView(scene: Fixtures.room, back: !wide, wide: wide)
    }

    public static func asking() -> RoomView {
        var round = RoomRound()
        round.asked(Fixtures.kestrel, named: [], when: .distantPast)
        round.began("chief")
        round.replied("chief")
        round.began("coder")
        round.began("res")
        round.asks("res", when: .distantPast)
        guard case .card(let id, _, let block) = Fixtures.cards[2] else { return kestrel() }
        let items: [Item] = [
            .you(id: 1, text: "Where is Kestrel's revenue heading? Use their last three 10-Qs."),
            .said(
                id: 2, speaker: "chief", time: "10:02",
                markdown: [.text(Markdown.inline("@Researcher has the filings; I'll wait for the numbers."))]),
            .card(id: id, poster: "res", block: block),
        ]
        return RoomView(
            scene: RoomScene(
                title: "Kestrel launch", members: Fixtures.kestrel.map(Fixtures.persona), others: [], round: round,
                items: items, inspecting: nil))
    }
}

extension RoomView {
    public static func mentioning() -> RoomView {
        var scene = Fixtures.room
        scene.round.replied("am")
        scene.round.ended("done")
        scene.items = [
            scene.items[3],
            .said(
                id: 5, speaker: "am", time: "09:42",
                markdown: [
                    .text(
                        Markdown.inline(
                            "Pricing: a 3-year term at the Birch & Co rate, 8% off list. The order form is in kestrel/order.pdf."
                        ))
                ]
            ),
            .divider(id: 6, text: RoomItems.ended(scene.round.turn, passed: ["Coder"])),
        ]
        scene.draft = "@Coder check Q3 with @Re"
        return RoomView(scene: scene)
    }
}

extension MemberSheet {
    public static func researcher() -> some View {
        var scene = Fixtures.room
        scene.inspecting = "res"
        scene.round.replied("am")
        return ZStack(alignment: .top) {
            RoomView(scene: scene)
            Color.black.opacity(0.45).ignoresSafeArea()
            MemberSheet(scene: scene, member: Fixtures.persona("res")).padding(.top, 150)
                .background(alignment: .top) { SheetCard().padding(.top, 150) }
        }
    }
}

extension NewRoomSheet {
    public static func kestrel() -> some View {
        NewRoomSheet(personas: Fixtures.personas, title: "Kestrel launch", picked: Fixtures.kestrel)
            .padding(.top, 8).background(alignment: .top) { SheetCard() }.padding(.top, 4)
    }
}

extension Sidebar where Column == EmptyView, Detail == RoomView {
    public static func kestrel() -> Sidebar<EmptyView, RoomView> {
        Sidebar(
            chats: Fixtures.roomChats, selected: "r1", actions: HomeActions(), column: { EmptyView() },
            detail: { RoomView.kestrel(wide: true) }
        )
    }
}

internal struct SheetCard: View {
    @Environment(\.look) private var look

    internal var body: some View {
        UnevenRoundedRectangle(topLeadingRadius: 34, topTrailingRadius: 34).fill(look.palette.background)
            .overlay(
                UnevenRoundedRectangle(topLeadingRadius: 34, topTrailingRadius: 34).fill(
                    look.palette.card.opacity(0.35))
            )
            .ignoresSafeArea()
    }
}
