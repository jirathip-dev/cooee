import CooeeCore
@testable import CooeeUI
import SwiftUI

public enum PanelState: String, CaseIterable, Sendable {
    case running, quiet, unreachable, unpaired
}

public enum ServerLink: Sendable {
    case running(String)
    case unreachable(String, seen: String)
    case unpaired
}

internal enum RowKind: Sendable {
    case ask, running
}

internal struct LiveRow: Identifiable, Sendable {
    internal let kind: RowKind
    internal let persona: Persona
    internal let title: String
    internal let detail: String
    internal let trailing: String
    internal var source: String?

    internal var id: String { title + detail }

    internal static let asks = [
        LiveRow(
            kind: .ask, persona: .named("res", .needs), title: "Researcher asks", detail: "Allow fetch on sec.gov?",
            trailing: "12m"),
        LiveRow(
            kind: .ask, persona: .named("cond", .needs), title: "cooee#659 · design", detail: "Approve the menu bar?",
            trailing: "4m", source: "drover"),
        LiveRow(
            kind: .ask, persona: .named("am", .fail), title: "cooee#618 · impl lane",
            detail: "blocked: needs a secret", trailing: "1h", source: "drover"),
    ]

    internal static let running = [
        LiveRow(
            kind: .running, persona: .named("coder", .working), title: "Coder", detail: "Export for Kestrel",
            trailing: "2:14"),
        LiveRow(
            kind: .running, persona: .named("chief", .working), title: "Chief", detail: "Q3 board deck",
            trailing: "0:48"),
        LiveRow(
            kind: .running, persona: .named("cond", .working), title: "cooee#631 · review", detail: "rev lane",
            trailing: "5:02", source: "drover"),
    ]
}

internal struct Snapshot: Sendable {
    internal let link: ServerLink
    internal let asks: [LiveRow]
    internal let running: [LiveRow]
    internal var asOf: String?

    internal static func of(_ state: PanelState) -> Snapshot {
        switch state {
        case .running: Snapshot(link: .running("mini"), asks: LiveRow.asks, running: LiveRow.running)
        case .quiet: Snapshot(link: .running("mini"), asks: [], running: [])
        case .unreachable:
            Snapshot(
                link: .unreachable("mini", seen: "9:12"), asks: Array(LiveRow.asks.prefix(1)),
                running: Array(LiveRow.running.prefix(1)), asOf: "9:12")
        case .unpaired: Snapshot(link: .unpaired, asks: [], running: [])
        }
    }
}

extension Persona {
    internal static func named(_ id: String, _ state: LaneState) -> Persona {
        Fixtures.persona(id).with(state: state)
    }
}
