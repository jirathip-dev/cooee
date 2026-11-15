import CooeeCore
import SwiftUI
import WebKit

internal struct LeafBlock: View {
    @Environment(\.look) private var look
    @Environment(\.card) private var context
    internal let block: JSONValue

    internal var body: some View {
        switch block.type {
        case "row": RowView(row: block)
        case "text": TextBlock(block: block)
        case "stat": StatBlock(block: block)
        case "chips": ChipsBlock(block: block)
        case "web": WebBlock(block: block)
        default: button
        }
    }

    private var button: some View {
        let style = block.string("style")
        let tint = style == "destructive" ? look.palette.red : context.tint
        return ActButton(source: block) {
            CardButton(title: block.string("label") ?? "", primary: style == "primary", tint: tint)
        }
    }
}

internal struct TextBlock: View {
    @Environment(\.look) private var look
    internal let block: JSONValue

    internal var body: some View {
        let markdown = block.string("markdown") ?? ""
        let until = block["until"]?.date
        TimelineView(.everyMinute) { context in
            let text = Relative.filled(markdown, until: until, at: context.date)
            if block.string("style") == "footnote" {
                Text(Markdown.inline(text)).ui(12, .regular, look).foregroundStyle(look.palette.mute)
            } else {
                VStack(alignment: .leading, spacing: 10) {
                    ForEach(Array(Markdown.blocks(text).enumerated()), id: \.offset) { _, part in BlockView(block: part)
                    }
                }
            }
        }
    }
}

internal struct ListBlock: View {
    @Environment(\.look) private var look
    internal let block: JSONValue

    internal var body: some View {
        let rows = block.list("rows")
        VStack(spacing: 0) {
            ForEach(Array(rows.enumerated()), id: \.offset) { index, row in
                RowView(row: row).modifier(Reordering(list: block, row: row))
                if index < rows.count - 1 {
                    Rectangle().fill(look.palette.line).frame(height: 0.5).padding(.leading, 12)
                }
            }
        }
        .surface(look, radius: 16)
    }
}

internal struct StatBlock: View {
    @Environment(\.look) private var look
    internal let block: JSONValue

    internal var body: some View {
        let tiles = block["items"] == nil ? [block] : block.list("items")
        HStack(spacing: 10) {
            ForEach(Array(tiles.enumerated()), id: \.offset) { _, tile in self.tile(tile) }
        }
    }

    private func tile(_ tile: JSONValue) -> some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 4) {
            Text(tile.string("label") ?? "").ui(12, .regular, look).foregroundStyle(palette.mute)
            HStack(alignment: .firstTextBaseline, spacing: 4) {
                RelativeValue(value: tile["value"] ?? .null).ui(22, .semibold, look)
                    .foregroundStyle(palette.tone(tile.string("tone")) ?? palette.foreground)
                if let unit = tile.string("unit") { Text(unit).ui(13, .regular, look).foregroundStyle(palette.mute) }
            }
            if !tile.list("spark").isEmpty {
                Sparkline(points: tile.list("spark").compactMap(\.number)).frame(height: 22)
            }
        }
        .padding(12).frame(maxWidth: .infinity, alignment: .leading)
        .surface(look, radius: 16)
    }
}

internal struct Sparkline: View {
    @Environment(\.look) private var look
    internal let points: [Double]

    internal var body: some View {
        GeometryReader { size in
            let low = points.min() ?? 0
            let span = max((points.max() ?? 0) - low, 1)
            let step = size.size.width / CGFloat(max(points.count - 1, 1))
            Path { path in
                for (index, point) in points.enumerated() {
                    let point = CGPoint(x: CGFloat(index) * step, y: size.size.height * (1 - (point - low) / span))
                    if index == 0 { path.move(to: point) } else { path.addLine(to: point) }
                }
            }
            .stroke(look.palette.lead, lineWidth: 1.5)
        }
    }
}

internal struct ChipsBlock: View {
    @Environment(\.look) private var look
    @Environment(\.act) private var act
    @State private var picked: [String]?
    internal let block: JSONValue

    internal var body: some View {
        let selected = picked ?? block.list("selected").compactMap(\.string)
        ScrollView(.horizontal) {
            HStack(spacing: 8) {
                ForEach(block.list("options").compactMap { $0.string("value") }, id: \.self) { value in
                    Button {
                        pick(value, from: selected)
                    } label: {
                        chip(value, selected: selected.contains(value))
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .scrollIndicators(.hidden)
    }

    private func chip(_ value: String, selected: Bool) -> some View {
        let label = block.list("options").first { $0.string("value") == value }?.string("label") ?? value
        return Text(label).ui(13, .semibold, look).foregroundStyle(
            selected ? look.palette.background : look.palette.foreground
        )
        .padding(.horizontal, 12).frame(height: 30)
        .background(selected ? look.palette.lead : look.palette.edge.opacity(0.35), in: .capsule)
    }

    private func pick(_ value: String, from selected: [String]) {
        let multi = block["multi"]?.bool == true
        let next = multi ? (selected.contains(value) ? selected.filter { $0 != value } : selected + [value]) : [value]
        picked = next
        let name = block.string("name") ?? ""
        let chosen: JSONValue = multi ? .array(next.map(JSONValue.string)) : .string(value)
        if let action = block.string("action") {
            act(BlockAction(action: action, values: [name: chosen]))
        }
    }
}

internal struct WebBlock: View {
    @State private var page: WebPage = {
        var configuration = WebPage.Configuration()
        configuration.websiteDataStore = .nonPersistent()
        return WebPage(configuration: configuration)
    }()
    internal let block: JSONValue

    internal var body: some View {
        if let url = block.string("src").flatMap(URL.init(string:)), url.scheme == "https" || url.scheme == "http" {
            WebView(page).frame(height: CGFloat(block["height"]?.number ?? 120))
                .onAppear { _ = page.load(URLRequest(url: url)) }
        }
    }
}
