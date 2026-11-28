import CooeeCore
import SwiftUI

internal struct TableCard: View {
    @Environment(\.look) private var look
    internal let table: TableBlock

    internal var body: some View {
        Sideways {
            if look.mono { ascii } else { grid.boxed(look) }
        }
    }

    private var grid: some View {
        Grid(alignment: .leading, horizontalSpacing: 0, verticalSpacing: 0) {
            GridRow { ForEach(table.header.indices, id: \.self) { cell(table.header[$0], column: $0, head: true) } }
            ForEach(table.rows.indices, id: \.self) { row in
                Rectangle().fill(look.palette.edge.opacity(0.7)).frame(height: 1).gridCellUnsizedAxes(.horizontal)
                GridRow { ForEach(table.rows[row].indices, id: \.self) { cell(table.rows[row][$0], column: $0) } }
            }
        }
    }

    private func cell(_ text: AttributedString, column: Int, head: Bool = false) -> some View {
        let palette = look.palette
        return Text(BlockView.styled(text, look)).font(.system(size: 14, weight: head ? .semibold : .regular))
            .foregroundStyle(head ? palette.foreground : palette.foreground.opacity(0.92))
            .lineLimit(1).padding(.horizontal, 12).padding(.vertical, 8)
            .frame(maxWidth: .infinity, alignment: table.trailing.contains(column) ? .trailing : .leading)
            .background(head ? palette.edge.opacity(0.35) : .clear)
    }

    private var widths: [Int] {
        table.header.indices.map { column in
            ([table.header] + table.rows).map { $0[column].characters.count }.max() ?? 0
        }
    }

    private var ascii: some View {
        let widths = widths
        func rule(_ left: String, _ mid: String, _ right: String) -> String {
            left + widths.map { String(repeating: "─", count: $0 + 2) }.joined(separator: mid) + right
        }
        func line(_ cells: [AttributedString]) -> String {
            "│"
                + cells.indices.map { column in
                    let text = String(cells[column].characters)
                    let pad = String(repeating: " ", count: widths[column] - text.count)
                    return " " + (table.trailing.contains(column) ? pad + text : text + pad) + " "
                }
                .joined(separator: "│") + "│"
        }
        var out = border(rule("┌", "┬", "┐") + "\n")
        var head = AttributedString(line(table.header) + "\n")
        head.foregroundColor = look.palette.lead
        head.font = .system(size: 13, weight: .bold, design: .monospaced)
        out += head + border(rule("├", "┼", "┤") + "\n")
        for row in table.rows { out += AttributedString(line(row) + "\n") }
        out += border(rule("└", "┴", "┘"))
        return Text(out).font(.system(size: 13, design: .monospaced)).foregroundStyle(look.palette.foreground)
    }

    private func border(_ text: String) -> AttributedString {
        var out = AttributedString(text)
        out.foregroundColor = look.palette.mute
        return out
    }
}

internal struct Sideways<Content: View>: View {
    @Environment(\.look) private var look
    @ViewBuilder internal let content: Content

    internal var body: some View {
        ViewThatFits(in: .horizontal) {
            content.fixedSize()
            Group {
                if look.still {
                    content.fixedSize().frame(minWidth: 0, maxWidth: .infinity, alignment: .leading).clipped()
                } else {
                    ScrollView(.horizontal, showsIndicators: false) { content.fixedSize() }
                }
            }
            .overlay(alignment: .trailing) {
                LinearGradient(
                    colors: [look.palette.card.opacity(0), look.palette.card], startPoint: .leading,
                    endPoint: .trailing
                )
                .frame(width: 32).allowsHitTesting(false)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

extension View {
    @ViewBuilder internal func boxed(_ look: Look) -> some View {
        let (palette, radius) = (look.palette, look.radius(12))
        if look.mono {
            background(palette.card.opacity(0.6))
                .overlay(Rectangle().strokeBorder(palette.edge, style: StrokeStyle(lineWidth: 1, dash: [3, 2])))
        } else {
            background(palette.card, in: .rect(cornerRadius: radius))
                .overlay(
                    RoundedRectangle(cornerRadius: radius).strokeBorder(palette.edge, lineWidth: look.wood ? 1.5 : 1)
                )
                .clipShape(.rect(cornerRadius: radius))
        }
    }
}
