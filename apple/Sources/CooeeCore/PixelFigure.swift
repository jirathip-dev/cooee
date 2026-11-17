import Foundation

/// A figure on its pixel grid: one or two frames of palette cells, row by row, nil where it is clear.
public struct PixelFigure: Equatable, Sendable {
    /// The colours a figure is painted with: the pack's figure colours, `--r --fg --bg`, white and near-black.
    public static let palette = [
        "--fk-l", "--fk-c", "--fk-a", "--fk-g", "--fk-p", "--r", "--fg", "--bg", "#ffffff", "#10161a",
    ]
    /// The states `sprites.svg` holds, then Scene's poses.
    public static let states =
        ["edit", "read", "run", "idle", "done", "needs", "merged", "fail", "loading", "empty"].map { "s-\($0)" }
        + ["edit", "idle", "done", "needs", "merged", "fail"].map { "s-scene-\($0)" }
    /// The backdrop's layers in `scene.svg`, bottom first, then the night look.
    public static let layers = ["sky", "sea", "rocks", "sand", "night"]

    public let width: Int
    public let height: Int
    public var frames: [[Int?]]

    /// A clear figure on the grid `id` takes: 24 × 24 for a state, else 132 × 100.
    public init(blank id: String) {
        let small = Self.states.prefix(8).contains(id)
        (width, height) = small ? (24, 24) : (132, 100)
        frames = [Array(repeating: nil, count: width * height)]
    }

    public func inside(_ col: Int, _ row: Int) -> Bool { (0..<width).contains(col) && (0..<height).contains(row) }

    public subscript(frame: Int, col: Int, row: Int) -> Int? {
        get { inside(col, row) && frames.indices.contains(frame) ? frames[frame][row * width + col] : nil }
        set { if inside(col, row), frames.indices.contains(frame) { frames[frame][row * width + col] = newValue } }
    }

    /// Paints the clear or same-coloured area around `col, row` with `colour`.
    public mutating func fill(_ frame: Int, _ col: Int, _ row: Int, _ colour: Int?) {
        let old = self[frame, col, row]
        guard inside(col, row), old != colour else { return }
        var todo = [(col, row)]
        while let (nextCol, nextRow) = todo.popLast() {
            guard inside(nextCol, nextRow), self[frame, nextCol, nextRow] == old else { continue }
            self[frame, nextCol, nextRow] = colour
            todo += [(nextCol + 1, nextRow), (nextCol - 1, nextRow), (nextCol, nextRow + 1), (nextCol, nextRow - 1)]
        }
    }

    /// The cells on the straight line from `from` to `end`.
    public static func line(_ from: [Int], _ end: [Int]) -> [[Int]] {
        guard from.count == 2, end.count == 2 else { return [end] }
        let (across, down) = (end[0] - from[0], end[1] - from[1])
        let steps = max(abs(across), abs(down), 1)
        return (0...steps).map { step in
            let part = Double(step) / Double(steps)
            return [from[0] + Int((Double(across) * part).rounded()), from[1] + Int((Double(down) * part).rounded())]
        }
    }

    /// The `#rrggbb` of palette colour `index` in `tokens`.
    public static func hex(_ index: Int, _ tokens: [String: String]) -> String? {
        palette.indices.contains(index) ? tokens[palette[index]] ?? palette[index] : nil
    }
}

extension PixelFigure {
    /// Frame `frame` as `<rect>` runs, a row's same-colour cells in one; a token falls back to its value in `tokens`.
    public func rects(_ frame: Int, _ tokens: [String: String]) -> String {
        var out = ""
        for row in 0..<height {
            var col = 0
            while col < width {
                let colour = self[frame, col, row]
                var run = 1
                while col + run < width, self[frame, col + run, row] == colour { run += 1 }
                if let colour, let hex = Self.hex(colour, tokens) {
                    let key = Self.palette[colour]
                    let fill = key.hasPrefix("#") ? "fill=\"\(key)\"" : "style=\"fill:var(\(key),\(hex))\""
                    out += "<rect x=\"\(col)\" y=\"\(row)\" width=\"\(run)\" height=\"1\" \(fill)/>"
                }
                col += run
            }
        }
        return out
    }

    /// The figure as `sprites.svg`'s `<symbol id>`, its frames as `.f1` and `.f2`.
    public func symbol(_ id: String, _ tokens: [String: String]) -> String {
        let body =
            frames.count == 1
            ? rects(0, tokens) : frames.indices.map { "<g class=\"f\($0 + 1)\">\(rects($0, tokens))</g>" }.joined()
        return "<symbol id=\"\(id)\" viewBox=\"0 0 \(width) \(height)\" shape-rendering=\"crispEdges\">\(body)</symbol>"
    }

    /// `scene.svg` of the backdrop's layers, each a `<g id>` of rect runs; `#night` shows only as the night look.
    public static func scene(_ layers: [String: PixelFigure], _ tokens: [String: String]) -> String {
        let groups = Self.layers.map {
            "<g id=\"\($0)\">\((layers[$0] ?? PixelFigure(blank: $0)).rects(0, tokens))</g>"
        }
        return
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 132 100\" preserveAspectRatio=\"xMidYMax slice\""
            + " shape-rendering=\"crispEdges\"><style>#night{display:none}#night:target{display:inline}</style>"
            + groups.joined() + "</svg>"
    }

    /// The figures in a pack's `sprites.svg` and `scene.svg` that are drawn as rect runs in the palette.
    public static func drawn(sprites: String, scene: String) -> [String: PixelFigure] {
        let found =
            states.map { ($0, sprites, "<symbol id=\"\($0)\"", "</symbol>") }
            + layers.map { ($0, scene, "<g id=\"\($0)\"", "</g>") }
        return found.reduce(into: [:]) { figures, item in
            let (id, svg, open, close) = item
            guard let start = svg.range(of: open), let end = svg[start.upperBound...].range(of: close) else { return }
            figures[id] = PixelFigure(id, String(svg[start.upperBound..<end.lowerBound]))
        }
    }

    private init?(_ id: String, _ body: String) {
        self.init(blank: id)
        for tag in body.split(separator: "<").dropFirst() {
            if tag.hasPrefix("/g") {
                continue
            } else if tag.hasPrefix("g class=\"f2\"") {
                frames.append(frames[0].map { _ in nil })
            } else if !tag.hasPrefix("g class=\"f1\""), !paint(tag) {
                return nil
            }
        }
    }

    private mutating func paint(_ tag: Substring) -> Bool {
        let value = { (name: String) in
            tag.range(of: " \(name)=\"").flatMap { found in tag[found.upperBound...].split(separator: "\"").first }
        }
        let number = { (name: String) in value(name).flatMap { Int($0) } }
        let key = value("style").flatMap { $0.split(separator: "(").last?.split(separator: ",").first } ?? value("fill")
        guard tag.hasPrefix("rect "), let col = number("x"), let row = number("y"), let run = number("width"),
            number("height") == 1, let colour = Self.palette.firstIndex(where: { $0 == key.map(String.init) })
        else { return false }
        for cell in stride(from: col, to: min(col + run, width), by: 1) { self[frames.count - 1, cell, row] = colour }
        return true
    }
}
