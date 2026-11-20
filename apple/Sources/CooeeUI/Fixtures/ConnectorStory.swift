import CooeeCore
import SwiftUI

extension Fixtures {
    private static let notion = #"""
        {"name": "notion", "label": "Notion", "publisher": "Notion", "source": "catalogue", "host": "mcp.notion.com",
         "leaves": "Pages and databases your personas search for or edit go to Notion.", "checked": "2026-11-20",
         "auth": "oauth", "accounts": [],
         "tools": {"read": ["search", "fetch", "get_comments", "get_users", "list_databases"],
                   "write": ["create_pages", "update_page", "create_comment", "move_pages"], "never": []}}
        """#

    private static let story = #"""
        {"connectors": [
          {"name": "linear", "label": "Linear", "publisher": "Linear", "source": "catalogue", "host": "mcp.linear.app",
           "leaves": "Issue text your personas search for or write goes to Linear.", "checked": "2026-11-20",
           "auth": "oauth",
           "tools": {"read": ["list_issues", "get_issue", "list_projects", "list_teams", "search_docs", "get_user"],
                     "write": ["create_issue", "update_issue", "create_comment"], "never": []},
           "accounts": [
             {"name": "main", "state": "signed_in", "since": 1795003235000, "personas": ["Chief", "Researcher"]},
             {"name": "work", "state": "signed_in", "since": 1795003235000, "personas": ["Coder"]}]},
          {"name": "github", "label": "GitHub", "publisher": "GitHub", "source": "catalogue",
           "host": "api.githubcopilot.com", "checked": "2026-11-20", "auth": "token",
           "leaves": "Repository names, issues and code your personas read go through GitHub.",
           "tools": {"read": ["search_code", "get_issue", "list_pull_requests"],
                     "write": ["create_issue", "merge_pull_request"], "never": ["delete_repository"]},
           "accounts": [{"name": "main", "state": "needs_sign_in", "why": "the token was revoked",
                         "since": 1795003235000, "personas": ["Coder"]}]},
          {"name": "acme", "label": "acme", "publisher": "", "source": "typed", "host": "mcp.acme.dev", "leaves": "",
           "auth": "oauth", "tools": {"read": [], "write": [], "never": []},
           "accounts": [{"name": "main", "state": "signed_in", "since": 1794484835000,
                         "personas": ["Account Manager"]}]},
          {"name": "files", "label": "files", "publisher": "", "source": "command", "host": "", "leaves": "",
           "auth": "none", "tools": {"read": [], "write": [], "never": []}, "accounts": []}],
         "catalogue": [NOTION,
          {"name": "calendar", "label": "Calendar", "publisher": "built into cooee", "source": "catalogue", "host": "",
           "leaves": "Read only: events in the range a persona asks for come back from Google.",
           "checked": "2026-11-20", "auth": "oauth", "accounts": [],
           "tools": {"read": ["list_events", "get_event", "free_busy"], "write": [], "never": []}}]}
        """#

    private static func decoded<T: Decodable>(_ json: String) -> T? {
        try? JSONDecoder().decode(T.self, from: Data(json.utf8))
    }

    /// The design's registry (#524): Linear, GitHub, a typed URL and a command, then Notion and Calendar to add.
    public static let connectors: ConnectorList =
        decoded(story.replacingOccurrences(of: "NOTION", with: notion)) ?? ConnectorList()

    /// Notion signed in as `main`.
    public static let notionSignedIn: Connector = {
        let since = Int64(Date().timeIntervalSince1970 * 1000)
        let main = #"{"name": "main", "state": "signed_in", "since": \#(since), "personas": []}"#
        let json = notion.replacingOccurrences(of: #""accounts": []"#, with: #""accounts": [\#(main)]"#)
        return decoded(json) ?? connector("notion")
    }()

    public static func connector(_ name: String) -> Connector {
        // SAFETY: every name a frame asks for is in `story`, and ConnectorFrames renders each one
        connectors.named(name)!
    }
}
