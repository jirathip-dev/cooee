import SwiftUI

internal struct TableView: View {
    @Environment(\.look) private var look
    internal let table: Table

    internal var body: some View {
        Group {
            if look.mono { ascii } else { grid.boxed(look) }
        }
        .fixedSize()
        .frame(width: look.column, alignment: .leading).clipped()
        .fadesRight(look, when: overflow)
    }

    private var overflow: Bool {
        let characters = widths.reduce(0, +) + widths.count * (look.mono ? 3 : 4)
        return !look.fits(characters, width: look.column)
    }

    private var widths: [Int] {
        table.header.indices.map { column in
            ([table.header] + table.rows).map { plain($0[column]).count }.max() ?? 0
        }
    }

    private var grid: some View {
        let palette = look.palette
        return Grid(alignment: .leading, horizontalSpacing: 0, verticalSpacing: 0) {
            GridRow { ForEach(table.header.indices, id: \.self) { cell(table.header[$0], column: $0, head: true) } }
            ForEach(table.rows.indices, id: \.self) { row in
                Rectangle().fill(palette.edge.opacity(0.7)).frame(height: 1).gridCellUnsizedAxes(.horizontal)
                GridRow { ForEach(table.rows[row].indices, id: \.self) { cell(table.rows[row][$0], column: $0) } }
            }
        }
    }

    private func cell(_ text: String, column: Int, head: Bool = false) -> some View {
        let alignment: Alignment = table.trailing.contains(column) ? .trailing : .leading
        return Text(Inline.text(text, look)).font(.system(size: 14, weight: head ? .semibold : .regular))
            .foregroundStyle(head ? look.palette.foreground : look.palette.foreground.opacity(0.92))
            .lineLimit(1).padding(.horizontal, 12).padding(.vertical, 8)
            .frame(maxWidth: .infinity, alignment: alignment)
            .background(head ? look.palette.edge.opacity(0.35) : .clear)
    }

    private var ascii: some View {
        let widths = widths
        func rule(_ left: String, _ mid: String, _ right: String) -> String {
            left + widths.map { String(repeating: "─", count: $0 + 2) }.joined(separator: mid) + right
        }
        func line(_ cells: [String]) -> String {
            "│" + cells.indices.map { column in
                let text = plain(cells[column])
                let pad = String(repeating: " ", count: widths[column] - text.count)
                return " " + (table.trailing.contains(column) ? pad + text : text + pad) + " "
            }.joined(separator: "│") + "│"
        }
        var out = border(rule("┌", "┬", "┐") + "\n")
        var head = AttributedString(line(table.header) + "\n")
        head.foregroundColor = look.palette.lead
        head.font = look.code(.bold)
        out += head + border(rule("├", "┼", "┤") + "\n")
        for row in table.rows { out += AttributedString(line(row) + "\n") }
        out += border(rule("└", "┴", "┘"))
        return Text(out).font(look.code()).foregroundStyle(look.palette.foreground)
    }

    private func border(_ text: String) -> AttributedString {
        var out = AttributedString(text)
        out.foregroundColor = look.palette.mute
        return out
    }

    private func plain(_ text: String) -> String { text.replacingOccurrences(of: "`", with: "") }
}
