import ComposableArchitecture
import CooeeCore
import Foundation

public struct Room: Equatable, Sendable {
    public let title: String
    public var members: [String]
    public let cast: [APIPersona]
    public var round = RoomRound()
    public var inspecting: String?
    public var asks: [RoomAsk] = []

    private static let cards = Int.min / 2

    public var names: [String: String] { Dictionary(cast.map { ($0.id, $0.name) }) { first, _ in first } }

    public init(title: String, members: [String], cast: [APIPersona]) {
        self.title = title
        self.members = members
        self.cast = cast
    }

    public var cards: [Item] {
        asks.enumerated().compactMap { index, ask in
            ask.item.block.map { .card(id: Self.cards + index, poster: ask.member, block: $0) }
        }
    }

    internal func ask(_ card: Int) -> RoomAsk? {
        asks.indices.contains(card - Self.cards) ? asks[card - Self.cards] : nil
    }

    internal mutating func heard(_ asks: [RoomAsk], when: Date) {
        self.asks = asks.filter(\.open)
        if let first = self.asks.first { round.asks(first.member, when: when) } else { round.answered(when: when) }
    }

    internal mutating func hear(_ event: StreamEvent, when: @autoclosure () -> Date) {
        switch event {
        case .roomAsk(let ask):
            heard(asks.filter { $0.session != ask.session || $0.item.id != ask.item.id } + [ask], when: when())
        case .roomTurn(let member):
            round.began(member)
        case .roomStop(let reason):
            round.ended(reason)
        case .item(let item) where item.kind == "you":
            round.asked(members, text: item.text ?? "", when: Date(timeIntervalSince1970: Double(item.time) / 1000))
        case .item(let item) where item.kind == "agent":
            if let speaker = item.speaker, !speaker.isEmpty { round.replied(speaker) }
        default:
            break
        }
    }
}

extension ChatFeature {
    @CasePathable
    public enum Seats: Sendable, Equatable {
        case inspect(String?)
        case move(String, Int)
        case seat(String)
        case unseat(String)
        case seated([String])
    }

    internal func seat(_ state: inout State, _ action: Seats) -> Effect<Action> {
        guard var room = state.room else { return .none }
        defer { state.room = room }
        switch action {
        case .inspect(let member):
            room.inspecting = member
        case .move(let member, let place):
            room.members = RoomOrder.move(room.members, member, to: place)
            return seated(state, member, place: place)
        case .seat(let persona):
            return seated(state, persona)
        case .unseat(let member):
            room.inspecting = nil
            return seated(state, member, remove: true)
        case .seated(let members):
            room.members = members
        }
        return .none
    }

    private func seated(_ state: State, _ persona: String, place: Int? = nil, remove: Bool = false) -> Effect<Action> {
        guard case .thread(let thread) = state.target else { return .none }
        let (threads, server) = (threads, state.server)
        return .run { send in
            await send(.room(.seated(try await threads.seat(server, thread, persona, place, remove))))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}
