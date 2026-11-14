import CooeeCore
import Foundation
import Synchronization

internal let mini = Server(url: URL(string: "https://mini.local") ?? URL(filePath: "/"), token: "tok")
internal let coder = APIPersona(id: "coder", name: "Coder", agent: "codex")
internal let conductor = APIPersona(id: "cond", name: "Conductor", agent: "codex", plugins: ["drover"])
internal let drover = PluginRow(
    name: "drover", version: "1", commit: "d", places: [], claims: ["drover"], permissions: PluginPermissions()
)
internal let epoch = Date(timeIntervalSince1970: 1_000_000)

internal final class Calls: Sendable {
    private let log = Mutex<[String]>([])

    internal var all: [String] { log.withLock { $0 } }

    internal func record(_ call: String) {
        log.withLock { $0.append(call) }
    }
}

internal final class Queue<Element: Sendable>: Sendable {
    private let items: Mutex<[Element]>

    internal init(_ items: [Element]) {
        self.items = Mutex(items)
    }

    internal func next() -> Element? {
        items.withLock { $0.isEmpty ? nil : $0.removeFirst() }
    }
}

internal func row(_ id: String, persona: String = "coder", state: String = "done", archived: Bool = false) -> ThreadRow
{
    ThreadRow(
        id: id, title: "T \(id)", agent: "codex", persona: persona, state: state, last: "", updated: 999_000_000,
        unseen: 0, archived: archived
    )
}

internal func lane(_ id: String, item: String = "#33", state: String = "working") -> ThreadRow {
    ThreadRow(
        id: id, title: "cooee \(item)", agent: "codex", persona: "", state: state, last: "", updated: 999_000_000,
        unseen: 0, archived: false,
        workflow: WorkflowLabel(source: "drover", group: "cooee", item: item, role: "impl", round: "1")
    )
}
