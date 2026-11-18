import CoreGraphics
import Foundation

/// A persona's browser as the Screen shows it (#162): who drives it and how many agent calls wait.
public struct BrowserScreen: Sendable, Equatable, Decodable {
    private enum CodingKeys: String, CodingKey { case control, waiting }
    private enum ControlKeys: String, CodingKey {
        case driver = "by"
        case until
    }

    public var until: Date?
    public var waiting: Int

    public init(until: Date? = nil, waiting: Int = 0) {
        self.until = until
        self.waiting = waiting
    }

    public init(from decoder: any Decoder) throws {
        let root = try decoder.container(keyedBy: CodingKeys.self)
        let control = try root.nestedContainer(keyedBy: ControlKeys.self, forKey: .control)
        let millis =
            try control.decode(String.self, forKey: .driver) == "owner"
            ? try control.decode(Double.self, forKey: .until) : nil
        until = millis.map { Date(timeIntervalSince1970: $0 / 1000) }
        waiting = try root.decode(Int.self, forKey: .waiting)
    }

    /// The seconds the owner's lease has left at `now`, or nil once it ended or while the agent drives.
    public func left(at now: Date) -> Int? {
        guard let until, until > now else { return nil }
        return Int(until.timeIntervalSince(now).rounded(.up))
    }
}

/// One frame of the live view: the current tab and its JPEG.
public struct LiveFrame: Sendable, Equatable, Decodable {
    private enum CodingKeys: String, CodingKey { case url, title, jpeg }

    public let url: String
    public let title: String
    public let jpeg: Data

    public init(url: String, title: String, jpeg: Data) {
        self.url = url
        self.title = title
        self.jpeg = jpeg
    }

    public init(from decoder: any Decoder) throws {
        let keys = try decoder.container(keyedBy: CodingKeys.self)
        url = try keys.decode(String.self, forKey: .url)
        title = try keys.decode(String.self, forKey: .title)
        jpeg = Data(base64Encoded: try keys.decode(String.self, forKey: .jpeg)) ?? Data()
    }

    /// The point on the page, in its CSS pixels, of a tap at `tap` in a view of `view` size that shows it fit.
    public static func page(_ tap: CGPoint, view: CGSize, page: CGSize) -> CGPoint {
        guard view.width > 0 else { return CGPoint(x: 0, y: 0) }
        let scale = page.width / view.width
        return CGPoint(x: (tap.x * scale).rounded(), y: (tap.y * scale).rounded())
    }
}

/// The owner's input while they hold control.
public enum BrowserInput: Sendable, Equatable, Encodable {
    case click(left: Double, top: Double)
    case text(String)

    private enum CodingKeys: String, CodingKey {
        case click, text
        case left = "x"
        case top = "y"
    }

    public func encode(to encoder: any Encoder) throws {
        var keys = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .click(let left, let top):
            var point = keys.nestedContainer(keyedBy: CodingKeys.self, forKey: .click)
            try point.encode(left, forKey: .left)
            try point.encode(top, forKey: .top)
        case .text(let text):
            try keys.encode(text, forKey: .text)
        }
    }
}

/// What the live view streams: the next frame, or why the browser isn't there.
public enum LiveEvent: Sendable, Equatable {
    case frame(LiveFrame)
    case gone(String)
}

/// A browser item in chat (#166): a frame the agent asked for, as a `screen` block, or a handoff to the owner (W5).
public enum ScreenItem: Sendable, Equatable {
    case shot(frame: String, address: String, time: Date)
    case handoff(what: String, waiting: Bool, note: String)

    public init?(_ item: APIItem) {
        switch item.kind {
        case "frame":
            guard let name = item.name else { return nil }
            self = .shot(frame: name, address: item.text ?? "", time: item.date)
        case "handoff":
            self = .handoff(what: item.text ?? "", waiting: item.outcome == "waiting", note: item.note ?? "")
        default:
            return nil
        }
    }
}

extension APIItem {
    /// When it happened.
    public var date: Date { Date(timeIntervalSince1970: Double(time) / 1000) }
}
