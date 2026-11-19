import CooeeCore
import Foundation

/// What the Mac menu bar panel draws (#659): the server's link, the chats waiting on the owner and those running.
public struct Glance: Sendable, Equatable {
    public enum Link: Sendable, Equatable {
        case running(String)
        case unreachable(String, since: String?)
        case unpaired
    }

    public static let shown = 3

    public var link: Link
    public var pair: URL?
    public var asks: [ListedChat]
    public var running: [ListedChat]

    public var stale: Bool {
        if case .unreachable = link { true } else { false }
    }

    public init(link: Link, pair: URL? = nil, asks: [ListedChat] = [], running: [ListedChat] = []) {
        self.link = link
        self.pair = pair
        self.asks = asks
        self.running = running
    }
}

extension AppFeature.State {
    /// Home's chats as the menu bar panel shows them: asks first, then those running, from the state Home keeps live.
    public func glance(now: Date) -> Glance {
        guard let home else { return Glance(link: .unpaired) }
        let url = home.server.url
        let name = url.host() ?? url.absoluteString
        let live = home.sync == .live && home.asOf == nil
        let since = home.asOf?.formatted(date: .omitted, time: .shortened)
        let chats = Listing.chats(rows: home.rows, personas: home.personas, now: now)
        return Glance(
            link: live ? .running(name) : .unreachable(name, since: since), pair: url.appending(path: "pair"),
            asks: chats.filter { $0.row.state == .needs }, running: chats.filter { $0.row.state == .working })
    }
}
