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
}

/// The Draw screen: the figures drawn so far, the one open, its frame and the pen.
public struct Drawing: Equatable, Sendable {
    public var figures: [String: PixelFigure] = [:]
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
        figures[open] = figure
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
    /// Puts `figures` in the draft: the states as `sprites.svg` symbols, the layers as `scene.svg`.
    public mutating func draw(_ figures: [String: PixelFigure]) {
        sprites = figures.filter { $0.key.hasPrefix("s-") }.reduce(into: [:]) {
            $0[$1.key] = $1.value.symbol($1.key, dark)
        }
        let layers = figures.filter { PixelFigure.layers.contains($0.key) }
        scene = layers.isEmpty ? nil : PixelFigure.scene(layers, dark)
    }
}
