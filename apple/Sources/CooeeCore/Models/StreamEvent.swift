public enum StreamEvent: Sendable, Equatable {
    case item(APIItem)
    case text(String)
    case state(Turn)
    case queue([Queued])
    case roomTurn(String)
    case roomStop(String)
    case reset
}
