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
        return ActButton(block: block.id ?? "", source: block) {
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
                Text(BlockView.marked(text)).ui(12, .regular, look).foregroundStyle(look.palette.mute)
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
                RowView(row: row)
                if index < rows.count - 1 {
                    Rectangle().fill(look.palette.line).frame(height: 0.5).padding(.leading, 12)
                }
            }
        }
        .surface(look, radius: 16)
    }
}

internal struct RowView: View {
    @Environment(\.look) private var look
    @Environment(\.act) private var act
    @State private var pending: JSONValue?
    internal let row: JSONValue

    internal var body: some View {
        let id = row.id ?? ""
        Group {
            if row["tap"] == nil {
                line
            } else {
                ActButton(block: id, source: row) { line }
            }
        }
        .contextMenu {
            ForEach(Array(row.list("swipe").enumerated()), id: \.offset) { _, swipe in
                Button(swipe.string("label") ?? "", role: swipe.string("tone") == "bad" ? .destructive : nil) {
                    if Confirm(for: swipe) == nil { run(swipe) } else { pending = swipe }
                }
            }
        }
        .modifier(Confirming(pending: $pending, run: run))
    }

    private var line: some View {
        let palette = look.palette
        let subtitle = row["subtitle"]
        return HStack(spacing: 10) {
            if let figure = row["figure"] { Figure(figure: figure) }
            VStack(alignment: .leading, spacing: 2) {
                Text(row.string("title") ?? "").ui(15, .medium, look)
                    .foregroundStyle(
                        row.string("style") == "destructive"
                            ? palette.red : row.string("style") == "link" ? palette.link : palette.foreground)
                if let text = subtitle?.string ?? subtitle?.string("text") {
                    Text(text).ui(12, .regular, look).foregroundStyle(
                        palette.tone(subtitle?.string("tone")) ?? palette.mute
                    ).lineLimit(1)
                }
            }
            Spacer(minLength: 4)
            if let badge = row["badge"] { Pill(badge: badge) }
            if let value = row["value"] {
                RelativeValue(value: value).ui(13, .regular, look).foregroundStyle(palette.mute)
            }
            if row["tap"] != nil && row.string("accessory") != "none" {
                Image(systemName: "chevron.right").font(.system(size: 11, weight: .bold)).foregroundStyle(palette.mute)
            }
        }
        .padding(.leading, CGFloat(row["indent"]?.number ?? 0) * 18).padding(.horizontal, 12).frame(minHeight: 44)
        .contentShape(.rect)
    }

    private func run(_ swipe: JSONValue) {
        if let action = BlockAction(block: row.id ?? "", action: nil, source: swipe, values: [:]) { act(action) }
    }
}

internal struct RelativeValue: View {
    internal let value: JSONValue

    internal var body: some View {
        TimelineView(.everyMinute) { context in Text(value.text(at: context.date) ?? "") }
    }
}

internal struct Figure: View {
    @Environment(\.look) private var look
    internal let figure: JSONValue

    internal var body: some View {
        let palette = look.palette
        if let icon = figure.string("icon") {
            Image(systemName: icon).font(.system(size: 14, weight: .semibold)).foregroundStyle(palette.link).frame(
                width: 28)
        } else if let avatar = figure.string("avatar") {
            Text(avatar.prefix(2).uppercased()).ui(12, .bold, look).frame(width: 28, height: 28)
                .background(palette.edge.opacity(0.5), in: .circle)
        } else {
            Circle().fill(palette.tone(figure.string("state")) ?? palette.mute).frame(width: 10, height: 10).frame(
                width: 28)
        }
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
            on ? look.palette.background : look.palette.foreground
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
            act(BlockAction(block: block.id ?? "", action: action, values: [name: chosen]))
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
