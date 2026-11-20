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
    public var clocks: [String: String]

    public var stale: Bool {
        if case .unreachable = link { true } else { false }
    }

    public init(
        link: Link, pair: URL? = nil, asks: [ListedChat] = [], running: [ListedChat] = [],
        clocks: [String: String] = [:]
    ) {
        self.link = link
        self.pair = pair
        self.asks = asks
        self.running = running
        self.clocks = clocks
    }

    /// A running chat's clock, minutes and seconds since its thread last changed.
    public static func clock(_ seconds: TimeInterval) -> String {
        Duration.seconds(max(0, seconds.rounded(.down))).formatted(.time(pattern: .minuteSecond))
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
        let clocks = home.rows.reduce(into: [String: String]()) { clocks, row in
            guard let updated = row.updated, LaneState(api: row.state) == .working else { return }
            clocks[row.id] = Glance.clock(now.timeIntervalSince1970 - TimeInterval(updated) / 1000)
        }
        return Glance(
            link: live ? .running(name) : .unreachable(name, since: since), pair: url.appending(path: "pair"),
            asks: chats.filter { $0.row.state == .needs }, running: chats.filter { $0.row.state == .working },
            clocks: clocks)
    }
}
