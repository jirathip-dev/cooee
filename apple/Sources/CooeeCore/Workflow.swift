import Foundation

public struct WorkflowLabel: Codable, Sendable, Equatable {
    public let source: String
    public let group: String
    public let item: String
    public let role: String
    public let round: String

    public var stage: String { "\(role) \(round)" }

    public init(source: String, group: String, item: String, role: String, round: String) {
        self.source = source
        self.group = group
        self.item = item
        self.role = role
        self.round = round
    }
}

public struct HomeLists: Sendable, Equatable {
    public var chats: [ListedChat] = []
    public var others: [ListedChat] = []
    public var workflow: [String: [ListedChat]] = [:]

    public var all: [ListedChat] { chats + others + workflow.values.flatMap(\.self) }

    public init() {}
}

public struct WorkflowIssue: Identifiable, Sendable, Equatable {
    public let id: String
    public let latest: ListedChat
    public let rounds: [ListedChat]

    public var needsYou: Bool { latest.row.state == .needs }
}

public enum Workflow {
    public static func issues(_ sessions: [ListedChat]) -> [WorkflowIssue] {
        let groups = Dictionary(grouping: sessions) { "\($0.workflow?.group ?? "") \($0.workflow?.item ?? "")" }
        let issues = groups.compactMap { id, rounds in
            rounds.first.map { WorkflowIssue(id: id, latest: $0, rounds: rounds.reversed()) }
        }
        return issues.sorted { (rank($0.latest.row.state), $0.id) < (rank($1.latest.row.state), $1.id) }
    }

    public static func groups(_ issues: [WorkflowIssue]) -> [(name: String, issues: [WorkflowIssue])] {
        ["needs you", "failed", "working", "ready"].enumerated().compactMap { rank, name in
            let members = issues.filter { Self.rank($0.latest.row.state) == rank }
            return members.isEmpty ? nil : (name, members)
        }
    }

    public static func others(_ chats: [ListedChat]) -> [(name: String, chats: [ListedChat])] {
        let sources = Set(chats.compactMap(\.workflow?.source)).sorted()
        let sections =
            [("chats · no persona", chats.filter { $0.workflow == nil })]
            + sources.map { source in
                ("\(source) · no plugin claims it", chats.filter { $0.workflow?.source == source })
            }
        return sections.filter { !$0.1.isEmpty }
    }

    private static func rank(_ state: LaneState) -> Int {
        [LaneState.needs, .fail, .working].firstIndex(of: state) ?? 3
    }
}

extension Listing {
    public static func home(rows: [ThreadRow], personas: [APIPersona], plugins: [PluginRow], now: Date) -> HomeLists {
        let owners = Dictionary(plugins.flatMap { plugin in plugin.claims.map { ($0, plugin.name) } }) { first, _ in
            first
        }
        var lists = HomeLists()
        for persona in personas where persona.plugins.contains(where: owners.values.contains) {
            lists.workflow[persona.id] = []
        }
        for (thread, listed) in zip(rows, chats(rows: rows, personas: personas, now: now)) {
            guard let source = thread.workflow?.source else {
                if personas.contains(where: { $0.id == thread.persona }) {
                    lists.chats.append(listed)
                } else {
                    lists.others.append(listed)
                }
                continue
            }
            let place = owners[source].flatMap { plugin in personas.first { $0.plugins.contains(plugin) } }
            if let place { lists.workflow[place.id, default: []].append(listed) } else { lists.others.append(listed) }
        }
        return lists
    }
}
