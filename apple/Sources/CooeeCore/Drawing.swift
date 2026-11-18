/// What a touch on the grid does.
public enum DrawTool: String, CaseIterable, Sendable {
    case pen, fill, erase, line
}

/// One change on the Draw screen.
public enum DrawEdit: Equatable, Sendable {
    case open(String?)
    case touch(col: Int, row: Int, began: Bool)
    case tool(DrawTool)
    case colour(Int)
    case mirror
    case frame(Int)
    case undo
    case cells([Int?])
    case vector(String)
}

/// The Draw screen: the figures drawn so far, the one open, its frame and the pen.
public struct Drawing: Equatable, Sendable {
    public var figures: [String: PixelFigure] = [:]
    /// Imported SVGs kept as vector, by id, each in place of its drawn figure.
    public var vectors: [String: String] = [:]
    public private(set) var open: String?
    public private(set) var frame = 0
    public private(set) var tool = DrawTool.pen
    public private(set) var colour = 0
    public private(set) var mirror = false
    private var undo: [PixelFigure?] = []
    private var from: [Int] = []

    public init() {}

    /// The open figure as drawn so far, else clear.
    public var figure: PixelFigure? { open.map { figures[$0] ?? PixelFigure(blank: $0) } }

    public mutating func apply(_ edit: DrawEdit) {
        switch edit {
        case .open(let id): (open, frame, undo) = (id, 0, [])
        case .touch(let col, let row, let began): touch(col, row, began)
        case .tool(let tool): self.tool = tool
        case .colour(let colour): self.colour = colour
        case .mirror: mirror.toggle()
        case .frame(let index): pick(index)
        case .undo: back()
        case .cells, .vector: paste(edit)
        }
    }

    private mutating func back() {
        guard let open, let was = undo.popLast() else { return }
        figures[open] = was
        frame = min(frame, (was?.frames.count ?? 1) - 1)
    }

    private mutating func pick(_ index: Int) {
        guard let open, var figure, (0...min(figure.frames.count, 1)).contains(index) else { return }
        if index == figure.frames.count, let last = figure.frames.last {
            undo.append(figures[open])
            figure.frames.append(last)
            figures[open] = figure
        }
        frame = index
    }

    private mutating func paste(_ edit: DrawEdit) {
        guard let open, var figure else { return }
        if case .vector(let svg) = edit { (vectors[open], figures[open]) = (svg, nil) }
        guard case .cells(let cells) = edit, cells.count == figure.width * figure.height else { return }
        undo.append(figures[open])
        figure.frames[frame] = cells
        (figures[open], vectors[open]) = (figure, nil)
    }

    private mutating func touch(_ col: Int, _ row: Int, _ began: Bool) {
        guard let open, var figure else { return }
        let paint = tool == .erase ? nil : colour
        if began {
            undo.append(figures[open])
            from = [col, row]
        }
        if tool == .fill {
            guard began else { return }
            figure.fill(frame, col, row, paint)
            if mirror { figure.fill(frame, figure.width - 1 - col, row, paint) }
        } else {
            stroke(&figure, open, [col, row], paint)
        }
        (figures[open], vectors[open]) = (figure, nil)
    }

    private mutating func stroke(_ figure: inout PixelFigure, _ open: String, _ cell: [Int], _ paint: Int?) {
        if tool == .line, let start = undo.last { figure = start ?? PixelFigure(blank: open) }
        for dot in PixelFigure.line(from, cell) {
            figure[frame, dot[0], dot[1]] = paint
            if mirror { figure[frame, figure.width - 1 - dot[0], dot[1]] = paint }
        }
        if tool != .line { from = cell }
    }
}

extension ThemeDraft {
    /// Puts `drawing` in the draft: the states as `sprites.svg` symbols, the layers as `scene.svg`.
    public mutating func draw(_ drawing: Drawing) {
        let ids = Set(drawing.figures.keys).union(drawing.vectors.keys)
        sprites = ids.filter { $0.hasPrefix("s-") }.reduce(into: [:]) { sprites, id in
            let box = PixelFigure(blank: id)
            sprites[id] =
                drawing.figures[id]?.symbol(id, dark)
                ?? "<symbol id=\"\(id)\" viewBox=\"0 0 \(box.width) \(box.height)\">\(drawing.vectors[id] ?? "")</symbol>"
        }
        let layered = ids.contains(where: PixelFigure.layers.contains)
        scene = layered ? PixelFigure.scene(drawing.figures, dark, vectors: drawing.vectors) : nil
    }
}
