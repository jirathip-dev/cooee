import SwiftUI

public enum AccountState: Sendable, Equatable {
    case signedIn(since: String)
    case needsSignIn(why: String)
    case signingIn
}

public struct Account: Sendable, Identifiable {
    public let name: String
    public let state: AccountState
    public let personas: [String]

    public var id: String { name }
}

public enum Source: Sendable, Equatable {
    case catalogue
    case typed
    case command
}

public struct Connector: Sendable, Identifiable {
    public let name: String
    public let label: String
    public let publisher: String
    public let host: String
    public let source: Source
    public let leaves: String
    public let reviewed: String
    public let read: [String]
    public let write: [String]
    public let never: [String]
    public let accounts: [Account]
    public let mark: String

    public var id: String { name }
}

public enum Story {
    public static let linear = Connector(
        name: "linear", label: "Linear", publisher: "Linear", host: "mcp.linear.app", source: .catalogue,
        leaves: "Issue text your personas search for or write goes to Linear.", reviewed: "20 Nov 2026",
        read: ["list_issues", "get_issue", "list_projects", "list_teams", "search_docs", "get_user"],
        write: ["create_issue", "update_issue", "create_comment"], never: [],
        accounts: [
            Account(name: "main", state: .signedIn(since: "18 Nov"), personas: ["Chief", "Researcher"]),
            Account(name: "work", state: .signedIn(since: "18 Nov"), personas: ["Coder"]),
        ],
        mark: "L"
    )

    public static let github = Connector(
        name: "github", label: "GitHub", publisher: "GitHub", host: "api.githubcopilot.com", source: .catalogue,
        leaves: "Repository names, issues and code your personas read go through GitHub.", reviewed: "20 Nov 2026",
        read: ["search_code", "get_issue", "list_pull_requests"], write: ["create_issue", "merge_pull_request"],
        never: ["delete_repository"],
        accounts: [Account(name: "main", state: .needsSignIn(why: "the token was revoked"), personas: ["Coder"])],
        mark: "G"
    )

    public static let acme = Connector(
        name: "acme", label: "acme", publisher: "", host: "mcp.acme.dev", source: .typed,
        leaves: "", reviewed: "", read: [], write: [], never: [],
        accounts: [Account(name: "main", state: .signedIn(since: "12 Nov"), personas: ["Account Manager"])],
        mark: "a"
    )

    public static let files = Connector(
        name: "files", label: "files", publisher: "", host: "this Mac", source: .command,
        leaves: "", reviewed: "", read: [], write: [], never: [], accounts: [], mark: "f"
    )

    public static let notion = Connector(
        name: "notion", label: "Notion", publisher: "Notion", host: "mcp.notion.com", source: .catalogue,
        leaves: "Pages and databases your personas search for or edit go to Notion.", reviewed: "20 Nov 2026",
        read: ["search", "fetch", "get_comments", "get_users", "list_databases"],
        write: ["create_pages", "update_page", "create_comment", "move_pages"], never: [],
        accounts: [], mark: "N"
    )

    public static let calendar = Connector(
        name: "calendar", label: "Calendar", publisher: "built into cooee", host: "googleapis.com",
        source: .catalogue, leaves: "Read only: events in the range a persona asks for come back from Google.",
        reviewed: "", read: ["list_events", "get_event", "free_busy"], write: [], never: [], accounts: [], mark: "C"
    )

    public static let connected = [linear, github, acme, files]
    public static let catalogue = [notion, calendar]
    public static let notionSigningIn = Connector(
        name: notion.name, label: notion.label, publisher: notion.publisher, host: notion.host, source: .catalogue,
        leaves: notion.leaves, reviewed: notion.reviewed, read: notion.read, write: notion.write, never: [],
        accounts: [Account(name: "main", state: .signingIn, personas: [])], mark: notion.mark
    )
    public static let notionSignedIn = Connector(
        name: notion.name, label: notion.label, publisher: notion.publisher, host: notion.host, source: .catalogue,
        leaves: notion.leaves, reviewed: notion.reviewed, read: notion.read, write: notion.write, never: [],
        accounts: [Account(name: "main", state: .signedIn(since: "just now"), personas: [])], mark: notion.mark
    )
}
