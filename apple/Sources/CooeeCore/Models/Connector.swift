import Foundation

/// A connector as `GET /connectors` lists it: a catalogue entry, or a registry server with its accounts.
public struct Connector: Codable, Sendable, Equatable, Identifiable {
    /// Where a registry server came from (#497 decision 1).
    public enum Source: String, Codable, Sendable {
        case catalogue, typed, command
    }

    /// Its tools as read (run), write (ask first) and never (not offered).
    public struct Tools: Codable, Sendable, Equatable {
        public let read: [String]
        public let write: [String]
        public let never: [String]
    }

    /// One signed-in (or needs sign-in) account and the personas that pick it.
    public struct Account: Codable, Sendable, Equatable, Identifiable {
        /// `signing_in` is the app's own while its sheet is open; cooee writes only the other two.
        public enum State: String, Codable, Sendable {
            case signedIn = "signed_in"
            case needsSignIn = "needs_sign_in"
            case signingIn = "signing_in"
        }

        public let name: String
        public var state: State
        public let why: String?
        public let since: Int64
        public let personas: [String]

        public var id: String { name }
    }

    public let name: String
    public let label: String
    public let publisher: String
    public let source: Source
    public let host: String
    public let leaves: String
    public let checked: String?
    public let auth: String
    public let tools: Tools
    public var accounts: [Account]

    public var id: String { name }
    public var signedIn: [Account] { accounts.filter { $0.state == .signedIn } }

    /// It with `account` shown as signing in, added when it has no account of that name yet.
    public func signingIn(_ account: String) -> Connector {
        let personas = accounts.first { $0.name == account }?.personas ?? []
        let pending = Account(name: account, state: .signingIn, why: nil, since: 0, personas: personas)
        var shown = self
        shown.accounts =
            accounts.contains { $0.name == account }
            ? accounts.map { $0.name == account ? pending : $0 } : accounts + [pending]
        return shown
    }
}

/// `GET /connectors`: the reviewed catalogue, then the registry's servers.
public struct ConnectorList: Codable, Sendable, Equatable {
    public let catalogue: [Connector]
    public let connectors: [Connector]

    /// The catalogue entries not in the registry yet.
    public var addable: [Connector] { catalogue.filter { entry in !connectors.contains { $0.name == entry.name } } }

    public init(catalogue: [Connector] = [], connectors: [Connector] = []) {
        self.catalogue = catalogue
        self.connectors = connectors
    }

    public func named(_ name: String) -> Connector? {
        connectors.first { $0.name == name } ?? catalogue.first { $0.name == name }
    }
}

/// `POST /connectors/{name}/signin`: the page to open in the sign-in sheet until a URL of scheme `until`, or the
/// token case's `help`.
public struct SignInStart: Codable, Sendable, Equatable {
    public let flow: String
    public let open: String?
    public let until: String?
    public let help: String?
}

/// `GET /connectors/flows/{id}`: how a sign-in ended.
public struct SignInFlow: Codable, Sendable, Equatable {
    public let state: String
    public let connector: String
    public let account: String
    public let why: String?

    public var signedIn: Bool { state == "signed_in" }
}

/// A persona's pick of a registry server (`[[mcp]]` in its `agent.toml`), with the account it calls as.
public struct MCPPick: Codable, Sendable, Equatable {
    public let server: String
    public let account: String?
    public let tools: [String]?

    public init(server: String, account: String?) {
        self.server = server
        self.account = account
        tools = nil
    }
}

extension APIPersona {
    /// Its pick of `server` set to `account`, in place; nil: detached.
    public mutating func pick(_ server: String, account: String?) {
        let picks = mcp ?? []
        let new = account.map { MCPPick(server: server, account: $0) }
        mcp =
            picks.contains { $0.server == server }
            ? picks.compactMap { $0.server == server ? new : $0 } : picks + [new].compactMap(\.self)
    }
}

extension Connector {
    /// The name of an account added beside the ones it has: `main`, then `work`, then `account-N`.
    public var nextAccount: String {
        let taken = Set(accounts.map(\.name))
        return (["main", "work"] + (3...99).map { "account-\($0)" }).first { !taken.contains($0) } ?? "main"
    }
}
