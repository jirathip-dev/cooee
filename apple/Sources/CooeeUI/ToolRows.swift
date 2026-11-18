import CooeeCore
import SwiftUI

extension ToolKind {
    internal var word: String {
        switch self {
        case .read: "Read"
        case .edit: "Edit"
        case .run: "Run"
        case .search: "Search"
        case .fetch: "Fetch"
        case .tool: "Tool"
        case .other: ""
        }
    }

    internal var symbol: String {
        switch self {
        case .read: "doc.text"
        case .edit: "pencil"
        case .run: "terminal"
        case .search: "magnifyingglass"
        case .fetch: "globe"
        case .tool: "puzzlepiece.extension"
        case .other: "sparkle"
        }
    }

    internal var ascii: String { self == .other ? "·" : rawValue }

    internal var path: Bool { [.read, .edit, .search].contains(self) }
}

extension ToolRow {
    internal var said: String { kind.word.isEmpty ? title : "\(kind.word) \(title)" }
}

internal struct ToolRowView: View {
    @Environment(\.look) private var look
    internal let row: ToolRow

    private var failed: Bool { row.status == "failed" }

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 8) {
            kind
            subject
            Spacer(minLength: 6)
            state
        }
        .padding(.horizontal, 12).frame(height: 34)
        .background(fill, in: .rect(cornerRadius: radius))
        .overlay {
            if look.mono {
                Rectangle().strokeBorder(failed ? palette.red.opacity(0.6) : palette.line, lineWidth: 1)
            } else if failed || look.style.stage != nil {
                RoundedRectangle(cornerRadius: radius).strokeBorder(
                    failed ? palette.red.opacity(0.55) : palette.edge, lineWidth: look.style.stage != nil ? 1.5 : 1)
            }
        }
    }

    private var radius: CGFloat { look.mono ? 0 : look.style.stage != nil ? 8 : 10 }

    private var fill: Color {
        let palette = look.palette
        if look.mono { return failed ? palette.red.opacity(0.08) : .clear }
        return failed ? palette.red.opacity(palette.dark ? 0.12 : 0.08) : palette.card.opacity(0.7)
    }

    @ViewBuilder private var kind: some View {
        let palette = look.palette
        if look.mono {
            Text(row.kind.ascii.padding(toLength: 6, withPad: " ", startingAt: 0))
                .font(.system(size: 13, weight: .bold, design: .monospaced)).foregroundStyle(palette.lead).fixedSize()
        } else {
            Image(systemName: row.kind.symbol).font(.system(size: 12, weight: .semibold))
                .foregroundStyle(palette.lead).frame(width: 18)
            if !row.kind.word.isEmpty {
                Text(row.kind.word).ui(13, .semibold, look).foregroundStyle(palette.foreground).fixedSize()
            }
        }
    }

    private var subject: some View {
        let mono = look.mono || row.kind != .other
        let text = row.kind == .run && look.mono ? "$ \(row.title)" : row.title
        return Text(text)
            .font(.system(size: mono ? 12.5 : 13, design: mono ? .monospaced : .default))
            .foregroundStyle(look.palette.mute).lineLimit(1).truncationMode(row.kind.path ? .head : .tail)
    }

    @ViewBuilder private var state: some View {
        let palette = look.palette
        if row.running {
            mark(ascii: "[…]", symbol: "circle.dotted", word: "running", tint: palette.cyan)
        } else if failed {
            mark(ascii: "[failed]", symbol: "xmark.octagon.fill", word: "failed", tint: palette.red)
        } else {
            mark(ascii: "[ok]", symbol: "checkmark.circle.fill", word: nil, tint: palette.green)
        }
    }

    @ViewBuilder private func mark(ascii: String, symbol: String, word: String?, tint: Color) -> some View {
        if look.mono {
            Text(ascii).font(.system(size: 12.5, weight: .bold, design: .monospaced)).foregroundStyle(tint)
                .fixedSize()
        } else {
            HStack(spacing: 5) {
                if let word { Text(word).ui(12, .medium, look) }
                Image(systemName: symbol).font(.system(size: 13))
            }
            .foregroundStyle(tint).fixedSize()
        }
    }
}

internal struct ToolFold: View {
    @Environment(\.look) private var look
    @Environment(\.members) private var members
    private let open: State<Bool>
    internal let rows: [ToolRow]

    internal init(rows: [ToolRow], open: Bool = false) {
        self.rows = rows
        self.open = State(initialValue: open)
    }

    internal var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            if let last = rows.last {
                Button {
                    open.wrappedValue.toggle()
                } label: {
                    HStack(spacing: 6) {
                        ToolRowView(row: last)
                        count
                    }
                    .contentShape(.rect)
                }
                .buttonStyle(.plain)
            }
            if open.wrappedValue {
                ForEach(Array(rows.dropLast().enumerated()), id: \.offset) { _, earlier in
                    ToolRowView(row: earlier).padding(.leading, look.mono ? 4 : 12)
                }
            }
        }
    }

    private var count: some View {
        let palette = look.palette
        let shown = open.wrappedValue
        return HStack(spacing: 4) {
            Text(look.mono ? "\(rows.count)" : "\(rows.count) tools").ui(12, .regular, look)
            if !members.isEmpty { Text("· only you see these").ui(12, .regular, look) }
            if look.mono {
                Text(shown ? "[v]" : "[>]").ui(12, .regular, look)
            } else {
                Image(systemName: shown ? "chevron.down" : "chevron.right").font(.system(size: 10, weight: .bold))
            }
        }
        .foregroundStyle(palette.mute).fixedSize()
    }
}
