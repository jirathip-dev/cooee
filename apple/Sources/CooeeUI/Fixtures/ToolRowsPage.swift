import CooeeCore
import SwiftUI

/// The tool-rows design page (#533): a chat with a closed and an open group, then every kind running, done and failed.
public struct ToolRowsPage: View {
    @Environment(\.look) private var look
    private let wide: Bool

    public var body: some View {
        let palette = look.palette
        ZStack(alignment: .top) {
            Backdrop()
            VStack(spacing: 0) {
                ChatHeader(persona: Fixtures.persona("coder"), chips: [], back: !wide).padding(.leading, wide ? 66 : 0)
                VStack(alignment: .leading, spacing: 14) {
                    ItemView(item: .you(id: 1, text: "The export test fails on CI. Can you fix it?", files: []))
                    ToolFold(rows: Self.closed)
                    ItemView(item: .agent(id: 3, markdown: [.text(Markdown.inline(Self.said))]))
                    ToolFold(rows: Self.open, open: true)
                    Rectangle().fill(palette.line).frame(height: 1).padding(.vertical, 10)
                    specimen
                }
                .padding(.horizontal, 16).padding(.vertical, 14).frame(width: wide ? 752 : nil)
            }
        }
        .foregroundStyle(palette.foreground)
    }

    public init(wide: Bool = false) {
        self.wide = wide
    }

    private var specimen: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(look.textMarks ? "── every kind: running, done, failed" : "Every kind: running, done, failed")
                .ui(12, .semibold, look).foregroundStyle(look.palette.mute).padding(.bottom, 2)
            ForEach(ToolKind.allCases, id: \.self) { kind in
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(["running", "done", "failed"], id: \.self) {
                        ToolRowView(row: ToolRow(kind, Self.sample(kind), $0))
                    }
                }
                .padding(.bottom, 6)
            }
        }
    }

    private static let said =
        "The test still reads `fixtures/q3.csv`, which moved to `data/`. Fixing the path, then running it."

    private static let closed = [
        ToolRow(.search, "crates/export", "done"),
        ToolRow(.read, "crates/export/tests/csv.rs", "done"),
        ToolRow(.read, "crates/export/tests/fixtures/q3.csv", "done"),
    ]

    private static let open = [
        ToolRow(.edit, "crates/export/tests/csv.rs", "done"),
        ToolRow(.run, "cargo test -p export", "failed"),
        ToolRow(.fetch, "docs.rs/csv/1.3.1/csv/struct.Reader.html", "done"),
        ToolRow(.tool, "cooee/remember", "done"),
        ToolRow(.other, "Plan the fix", "done"),
        ToolRow(.edit, "crates/export/src/csv.rs", "done"),
        ToolRow(.run, "cargo test -p export -- --nocapture", "running"),
    ]

    private static func sample(_ kind: ToolKind) -> String {
        switch kind {
        case .read, .edit: "crates/export/src/csv.rs"
        case .run: "cargo test -p export"
        case .search: "crates/export"
        case .fetch: "docs.rs/csv/1.3.1"
        case .tool: "cooee/remember"
        case .other: "Plan the fix"
        }
    }
}
