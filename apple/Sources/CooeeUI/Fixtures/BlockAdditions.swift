import CooeeCore
import SwiftUI

/// A plugin screen showing one of #315's block additions mid-use, for the design frames.
public struct BlockAdditions: View {
    /// The addition a frame shows.
    public enum Addition: String, CaseIterable, Sendable {
        case swipe, menu, refresh, transition
    }

    @Environment(\.look) private var look
    private let addition: Addition

    public var body: some View {
        ZStack(alignment: .top) {
            blocks(Self.queue(lifted: addition == .menu))
                .environment(\.swiped, addition == .swipe ? "q-2" : nil)
                .environment(\.refresh, addition == .refresh ? Self.refresh : nil)
            if addition == .menu { menu }
            if addition == .transition {
                blocks(Self.lane).frame(maxHeight: .infinity, alignment: .top).background(look.palette.background)
                    .modifier(Entering(transition: .sheet, shown: 0.55))
            }
        }
        .padding(.top, addition == .refresh ? 28 : 0).frame(maxHeight: .infinity, alignment: .top)
    }

    public init(_ addition: Addition) {
        self.addition = addition
    }

    private var menu: some View {
        VStack(alignment: .leading, spacing: 12) {
            MenuItems(acts: Self.acts["menu"]?.items ?? []) { _ in }
        }
        .ui(15, .regular, look).buttonStyle(.plain).padding(16).frame(width: 220, alignment: .leading)
        .surface(look, radius: 14)
        .background(look.palette.background, in: .rect(cornerRadius: look.radius(14))).padding(.top, 214)
        .padding(.leading, 40)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func blocks(_ screen: JSONValue) -> some View {
        BlocksView(screen, source: .plugin("drover")).padding(BlocksView.inset)
    }

    private static func refresh() {}

    private static let acts = JSONValue(
        json: """
            {"swipe": [{"label": "Pin", "action": "pin"}, {"label": "Stop", "tone": "bad", "action": "stop"}],
             "menu": [{"label": "Open lane", "action": "open"}, {"label": "Re-run review", "action": "rerun"},
                      {"label": "Stop", "tone": "bad", "action": "stop"}]}
            """)

    private static func row(_ id: String, _ title: String, _ stage: String, _ state: String, lifted: Bool) -> JSONValue
    {
        .object([
            "type": .string("row"), "id": .string(id), "title": .string(title),
            "subtitle": .string(stage), "figure": .object(["state": .string(state)]),
            "swipe": acts["swipe"] ?? .null, "menu": acts["menu"] ?? .null,
            "lifted": .bool(lifted && id == "q-2"),
        ])
    }

    private static func queue(lifted: Bool) -> JSONValue {
        let row = { (id: String, title: String, stage: String, state: String) in
            Self.row(id, title, stage, state, lifted: lifted)
        }
        return .object([
            "type": .string("screen"), "title": .string("Queue"), "refresh": .bool(true),
            "actions": .array([
                .object(["label": .string("More"), "icon": .string("more"), "menu": acts["menu"] ?? .null])
            ]),
            "blocks": .array([
                .object([
                    "type": .string("list"),
                    "rows": .array([
                        row("q-1", "#722 cap raise", "impl · working", "working"),
                        row("q-2", "#315 block additions", "review 1 · needs you", "needs"),
                        row("q-3", "#314 persona panel", "merged", "done"),
                    ]),
                ]),
                .object([
                    "type": .string("card"), "title": .string("Merge #314?"),
                    "subtitle": .string("review PASS · CI green"),
                    "menu": acts["menu"] ?? .null,
                ]),
            ]),
        ])
    }

    private static let lane = JSONValue(
        json: """
            {"type": "screen", "title": "#315 lane", "large": false, "blocks": [
              {"type": "text", "markdown": "Review 1 asks for a frame per addition in each style."},
              {"type": "list", "rows": [{"type": "row", "title": "impl", "subtitle": "done"},
                                        {"type": "row", "title": "review 1", "subtitle": "needs you"}]}]}
            """)
}
