import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct ThemeDraw: View {
    private static let edits = DrawTool.allCases.map(DrawEdit.tool) + [.mirror, .undo]
    private static let icons = [
        "pen": "pencil", "fill": "drop", "erase": "eraser", "line": "line.diagonal",
        "mirror": "arrow.left.and.right.righttriangle.left.righttriangle.right", "undo": "arrow.uturn.backward",
    ]

    @Environment(\.look) private var look
    @State private var stroke = false
    internal let store: StoreOf<ThemeEditorFeature>

    private var drawing: Drawing { store.drawing }
    private var tokens: [String: String] { store.state.tokens(dark: look.palette.dark) }

    internal var body: some View {
        let figure = drawing.figure ?? PixelFigure(blank: "s-edit")
        let shown = ThemePreview.look(store.state, look, dark: look.palette.dark)
        VStack(spacing: 12) {
            strip.environment(\.look, shown)
            grid(figure, shown)
            Text("\(figure.width) × \(figure.height) · frame \(drawing.frame + 1) of \(figure.frames.count)")
                .ui(14, .regular, look).foregroundStyle(look.palette.mute).frame(
                    maxWidth: .infinity, alignment: .leading)
            frames(figure).environment(\.look, shown)
            tools
            swatches
            ForEach([true, false], id: \.self) { dark in
                ThemePreview(state: store.state, chat: false, dark: dark, pose: pose)
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 16).padding(.top, 8)
        .background(look.palette.background).foregroundStyle(look.palette.foreground)
        .navigationTitle(title)
        .toolbar { Button("Done") { store.send(.draw(.open(nil))) } }
    }

    private var pose: String { String((drawing.open ?? "").split(separator: "-").last ?? "") }

    private var title: String {
        let open = drawing.open ?? ""
        return store.words[pose].map { "\(open) · \($0)" } ?? open
    }

    private var strip: some View {
        let ids = PixelFigure.layers.contains(drawing.open ?? "") ? PixelFigure.layers : PixelFigure.states
        return Scroller(axis: .horizontal) {
            HStack(spacing: 4) {
                ForEach(ids, id: \.self) { id in
                    let label = id.hasPrefix("s-") ? String(id.dropFirst(2)) : id
                    tile(label, picked: id == drawing.open) {
                        store.send(.draw(.open(id)))
                    } art: {
                        Sprite(name: "horse-bay-idle", pack: "sprites.svg#\(id)")
                    }
                }
            }
        }
        .frame(height: 56)
    }

    private func frames(_ figure: PixelFigure) -> some View {
        HStack(spacing: 8) {
            ForEach(figure.frames.indices, id: \.self) { index in
                tile("\(index + 1)", picked: index == drawing.frame) {
                    store.send(.draw(.frame(index)))
                } art: {
                    FigureArt(figure: figure, frame: index)
                }
            }
            if figure.frames.count == 1 {
                tile("", picked: false) {
                    store.send(.draw(.frame(1)))
                } art: {
                    Image(systemName: "plus")
                }
            }
            Spacer()
            Text(figure.frames.count == 2 ? "loops · 0.6 s\nonion skin on" : "one frame")
                .ui(15, .regular, look).foregroundStyle(look.palette.mute).multilineTextAlignment(.trailing)
        }
    }

    private var tools: some View {
        HStack(spacing: 0) {
            ForEach(Self.edits.indices, id: \.self) { index in
                let (edit, label) = (Self.edits[index], Self.label(Self.edits[index]))
                let picked = edit == .tool(drawing.tool) || (edit == .mirror && drawing.mirror)
                Button {
                    store.send(.draw(edit))
                } label: {
                    VStack(spacing: 4) {
                        if !look.mono { Image(systemName: Self.icons[label] ?? "").font(.system(size: 18)) }
                        Text(label).ui(look.mono ? 14 : 12, .regular, look)
                    }
                    .frame(maxWidth: .infinity, minHeight: 48)
                    .background(picked ? look.palette.lead : .clear, in: .rect(cornerRadius: 12))
                    .foregroundStyle(picked ? look.palette.background : look.palette.foreground)
                }
                .buttonStyle(.plain).accessibilityAddTraits(picked ? .isSelected : [])
            }
        }
        .padding(6).background(look.palette.card, in: .rect(cornerRadius: 16))
    }

    private var swatches: some View {
        HStack(spacing: 6) {
            ForEach(PixelFigure.palette.indices, id: \.self) { index in
                Button {
                    store.send(.draw(.colour(index)))
                } label: {
                    Circle().fill(Color.figure(PixelFigure.hex(index, tokens))).overlay {
                        Circle().stroke(look.palette.edge)
                    }
                    .padding(3).overlay {
                        Circle().stroke(index == drawing.colour ? look.palette.lead : .clear, lineWidth: 2)
                    }
                }
                .buttonStyle(.plain).accessibilityLabel(PixelFigure.palette[index])
            }
        }
    }

    private func grid(_ figure: PixelFigure, _ shown: Look) -> some View {
        let under = PixelFigure.layers.prefix { $0 != drawing.open }.compactMap { drawing.figures[$0] }
        return GeometryReader { space in
            let cell = space.size.width / CGFloat(figure.width)
            ZStack {
                ForEach(under.indices, id: \.self) { FigureArt(figure: under[$0], frame: 0).opacity(0.35) }
                if drawing.frame == 1 { FigureArt(figure: figure, frame: 0).opacity(0.22) }
                FigureArt(figure: figure, frame: drawing.frame, lines: figure.width <= 24)
            }
            .environment(\.look, shown).contentShape(.rect)
            .gesture(
                DragGesture(minimumDistance: 0).onChanged { touch in
                    let (col, row) = (Int(touch.location.x / cell), Int(touch.location.y / cell))
                    store.send(.draw(.touch(col: col, row: row, began: !stroke)))
                    stroke = true
                }
                .onEnded { _ in stroke = false }
            )
        }
        .aspectRatio(CGFloat(figure.width) / CGFloat(figure.height), contentMode: .fit)
        .background(look.palette.card).accessibilityLabel("Grid")
    }
}

extension ThemeDraw {
    private static func label(_ edit: DrawEdit) -> String {
        switch edit {
        case .tool(let tool): tool.rawValue
        case .mirror: "mirror"
        default: "undo"
        }
    }

    private func tile(
        _ label: String, picked: Bool, tap: @escaping () -> Void, @ViewBuilder art: () -> some View
    ) -> some View {
        Button(action: tap) {
            VStack(spacing: 4) {
                art().frame(width: 30, height: 22)
                if !label.isEmpty { Text(label).ui(11, .regular, look).lineLimit(1).minimumScaleFactor(0.6) }
            }
            .frame(width: 41, height: 52).background(look.palette.card, in: .rect(cornerRadius: 12))
            .overlay { RoundedRectangle(cornerRadius: 12).stroke(picked ? look.palette.lead : .clear, lineWidth: 2) }
        }
        .buttonStyle(.plain).accessibilityAddTraits(picked ? .isSelected : [])
    }
}

internal struct FigureArt: View {
    @Environment(\.look) private var look
    internal let figure: PixelFigure
    internal var frame: Int?
    internal var lines = false

    internal var body: some View {
        if frame == nil, figure.frames.count > 1, !look.still {
            TimelineView(.periodic(from: .now, by: 0.6)) { context in
                canvas(Int(context.date.timeIntervalSinceReferenceDate / 0.6) % 2)
            }
        } else {
            canvas(frame ?? 0)
        }
    }

    private func canvas(_ frame: Int) -> some View {
        let (figure, tokens, edge) = (figure, look.tokens, look.palette.line)
        return Canvas { context, size in
            let cell = size.width / CGFloat(figure.width)
            for row in 0..<figure.height {
                for col in 0..<figure.width {
                    let box = CGRect(x: CGFloat(col) * cell, y: CGFloat(row) * cell, width: cell, height: cell)
                    if let colour = figure[frame, col, row] {
                        context.fill(Path(box), with: .color(.figure(PixelFigure.hex(colour, tokens))))
                    }
                    if lines { context.stroke(Path(box), with: .color(edge), lineWidth: 0.5) }
                }
            }
        }
        .aspectRatio(CGFloat(figure.width) / CGFloat(figure.height), contentMode: .fit)
    }
}

extension Color {
    internal static func figure(_ hex: String?) -> Color { .hex(UInt32(hex?.dropFirst() ?? "", radix: 16) ?? 0) }
}
