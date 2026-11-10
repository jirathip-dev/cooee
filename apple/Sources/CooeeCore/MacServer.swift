import Foundation

public enum ServiceState: String, Sendable, Equatable {
    case enabled = "on"
    case off
    case needsApproval = "waiting for approval in Login Items"
    case missing = "not found"
}

public struct LoginItems: Sendable, Equatable {
    public let cooee: ServiceState
    public let shed: ServiceState

    public var summary: String { "cooee \(cooee.rawValue) · shed \(shed.rawValue)" }

    public init(cooee: ServiceState, shed: ServiceState) {
        self.cooee = cooee
        self.shed = shed
    }
}

public struct FoundAgent: Sendable, Equatable {
    public static let known = ["claude": "Claude Code", "codex": "Codex", "hermes": "Hermes"]
    public static let order = ["claude", "codex", "hermes"]

    public let name: String
    public let version: String
    public let path: String?

    public var title: String { Self.title(name) }
    public var detail: String {
        path.map { [version, $0].filter { !$0.isEmpty }.joined(separator: " · ") } ?? "not installed · optional"
    }

    public init(name: String, version: String = "", path: String? = nil) {
        self.name = name
        self.version = version
        self.path = path
    }

    public static func title(_ name: String) -> String { known[name] ?? name }

    /// Every known agent from `name<TAB>path<TAB>version` lines, in `order`; one with no line is not installed.
    public static func list(found lines: String) -> [FoundAgent] {
        let rows = lines.split(separator: "\n").map { $0.split(separator: "\t", omittingEmptySubsequences: false) }
        return order.map { name in
            guard let row = rows.first(where: { $0.first.map(String.init) == name }), row.count > 1 else {
                return FoundAgent(name: name)
            }
            let words = row.count > 2 ? row[2].split(separator: " ") : []
            let version = words.first { $0.first?.isNumber == true }.map(String.init) ?? ""
            return FoundAgent(name: name, version: version, path: String(row[1]))
        }
    }
}

public struct SignIn: Sendable, Equatable, Decodable {
    private enum CodingKeys: String, CodingKey {
        case provider, name
        case signedIn = "signed_in"
    }

    public let provider: String
    public let name: String
    public let signedIn: Bool

    public var detail: String { signedIn ? (name.isEmpty ? "signed in" : name) : "not signed in" }

    public init(provider: String, name: String, signedIn: Bool) {
        self.provider = provider
        self.name = name
        self.signedIn = signedIn
    }
}

public struct Network: Sendable, Equatable {
    public let name: String
    public let lan: String
    public let tailnet: String?

    public init(name: String, lan: String, tailnet: String?) {
        self.name = name
        self.lan = lan
        self.tailnet = tailnet
    }

    /// The tailnet name in `tailscale status --json`, if it is signed in.
    public static func tailnet(status: Data) -> String? {
        let name = (try? JSONDecoder().decode(TailscaleStatus.self, from: status))?.peer?.dnsName ?? ""
        let trimmed = name.hasSuffix(".") ? String(name.dropLast()) : name
        return trimmed.isEmpty ? nil : trimmed
    }
}

private struct TailscaleStatus: Decodable {
    private enum CodingKeys: String, CodingKey {
        case peer = "Self"
    }

    fileprivate let peer: TailscalePeer?
}

private struct TailscalePeer: Decodable {
    private enum CodingKeys: String, CodingKey {
        case dnsName = "DNSName"
    }

    fileprivate let dnsName: String
}

public struct PairCode: Sendable, Equatable {
    public let link: URL
    public let code: String
    public let minutes: Int

    public var host: String { link.host() ?? "" }
    public var local: URL { URL(string: "http://127.0.0.1:\(link.port ?? 4747)") ?? link }

    public init(link: URL, code: String, minutes: Int) {
        self.link = link
        self.code = code
        self.minutes = minutes
    }

    /// The code `cooee pair` printed: its `code  123 456  (one use, 10 minutes)` and `link  <url>` lines.
    public init?(output: String) {
        let lines = output.split(separator: "\n").map(String.init)
        guard
            let link = lines.first(where: { $0.hasPrefix("link  ") }).flatMap({ URL(string: String($0.dropFirst(6))) }),
            let code = PairTarget(typed: link.absoluteString)?.code
        else { return nil }
        let line = lines.first { $0.hasPrefix("code  ") } ?? ""
        let minutes = line.split(separator: ",").last?.split(separator: " ").first.flatMap { Int($0) }
        self.init(link: link, code: code, minutes: minutes ?? 10)
    }
}

public struct PairRequest: Sendable, Equatable, Decodable {
    public let id: String
    public let word: String
    public let label: String

    public init(id: String, word: String, label: String) {
        self.id = id
        self.word = word
        self.label = label
    }
}
