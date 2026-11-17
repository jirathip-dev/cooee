@testable import CooeeCore
import Foundation
import Testing

internal struct DrawingTests {
    private static let tokens = ["--fk-l": "#ff9e80", "--fk-c": "#4fd1c5"]

    private func drawing(_ edits: DrawEdit...) -> Drawing {
        var drawing = Drawing()
        drawing.apply(.open("s-needs"))
        for edit in edits { drawing.apply(edit) }
        return drawing
    }

    private func cells(_ drawing: Drawing, frame: Int = 0) -> [[Int]] {
        guard let figure = drawing.figure else { return [] }
        return (0..<24).flatMap { row in (0..<24).compactMap { col in figure[frame, col, row].map { [col, row, $0] } } }
    }

    @Test internal func aStateIs24SquareAndTheRestAre132By100() {
        #expect(PixelFigure(blank: "s-needs").width == 24 && PixelFigure(blank: "s-needs").height == 24)
        for id in ["s-loading", "s-empty", "s-scene-needs", "sky"] {
            #expect(PixelFigure(blank: id).width == 132 && PixelFigure(blank: id).height == 100)
        }
    }

    @Test internal func thePenJoinsItsTouchesAndMirrorPaintsBothSides() {
        let pen = drawing(.colour(1), .touch(col: 0, row: 0, began: true), .touch(col: 3, row: 0, began: false))
        #expect(cells(pen) == [[0, 0, 1], [1, 0, 1], [2, 0, 1], [3, 0, 1]])
        let mirrored = drawing(.mirror, .touch(col: 2, row: 5, began: true))
        #expect(cells(mirrored) == [[2, 5, 0], [21, 5, 0]] && mirrored.mirror)
        #expect(drawing(.touch(col: 30, row: 2, began: true)).figure == PixelFigure(blank: "s-needs"))
        #expect(cells(drawing(.touch(col: 1, row: 1, began: false))) == [[1, 1, 0]])
    }

    @Test internal func fillEraseAndLineDoWhatTheySay() {
        let filled = drawing(.touch(col: 0, row: 1, began: true), .tool(.fill), .touch(col: 0, row: 0, began: true))
        #expect(cells(filled).count == 24 * 24 && filled.tool == .fill)
        let erased = drawing(
            .touch(col: 0, row: 0, began: true), .touch(col: 1, row: 0, began: false), .tool(.erase),
            .touch(col: 0, row: 0, began: true))
        #expect(cells(erased) == [[1, 0, 0]])
        let line = drawing(
            .tool(.line), .colour(2), .touch(col: 0, row: 0, began: true), .touch(col: 5, row: 5, began: false),
            .touch(col: 2, row: 0, began: false))
        #expect(cells(line) == [[0, 0, 2], [1, 0, 2], [2, 0, 2]])
    }

    @Test internal func aSecondFrameCopiesTheFirstAndUndoTakesBackEachStep() {
        var drawing = drawing(.touch(col: 4, row: 4, began: true), .frame(1), .touch(col: 5, row: 4, began: true))
        #expect(drawing.frame == 1 && drawing.figure?.frames.count == 2)
        #expect(cells(drawing, frame: 1) == [[4, 4, 0], [5, 4, 0]] && cells(drawing) == [[4, 4, 0]])
        drawing.apply(.frame(2))
        #expect(drawing.frame == 1)
        drawing.apply(.undo)
        drawing.apply(.undo)
        #expect(drawing.frame == 0 && drawing.figure?.frames.count == 1)
        drawing.apply(.undo)
        drawing.apply(.undo)
        #expect(drawing.figures.isEmpty)
        drawing.apply(.open(nil))
        drawing.apply(.touch(col: 1, row: 1, began: true))
        drawing.apply(.frame(1))
        #expect(drawing.figure == nil && drawing.figures.isEmpty)
    }

    @Test internal func aFigureIsOneSymbolOfRectRunsAndReadsBack() {
        let drawing = drawing(
            .touch(col: 2, row: 3, began: true), .touch(col: 4, row: 3, began: false), .colour(8),
            .touch(col: 5, row: 3, began: true), .frame(1), .colour(1), .touch(col: 0, row: 0, began: true))
        let figure = drawing.figure ?? PixelFigure(blank: "s-needs")
        let symbol = figure.symbol("s-needs", Self.tokens)
        #expect(
            symbol == "<symbol id=\"s-needs\" viewBox=\"0 0 24 24\" shape-rendering=\"crispEdges\"><g class=\"f1\">"
                + "<rect x=\"2\" y=\"3\" width=\"3\" height=\"1\" style=\"fill:var(--fk-l,#ff9e80)\"/>"
                + "<rect x=\"5\" y=\"3\" width=\"1\" height=\"1\" fill=\"#ffffff\"/></g><g class=\"f2\">"
                + "<rect x=\"0\" y=\"0\" width=\"1\" height=\"1\" style=\"fill:var(--fk-c,#4fd1c5)\"/>"
                + "<rect x=\"2\" y=\"3\" width=\"3\" height=\"1\" style=\"fill:var(--fk-l,#ff9e80)\"/>"
                + "<rect x=\"5\" y=\"3\" width=\"1\" height=\"1\" fill=\"#ffffff\"/></g></symbol>")
        let sheet =
            "<svg><style>.f2{opacity:0}</style>\(symbol)<symbol id=\"s-read\" viewBox=\"0 0 24 24\">"
            + "<rect x=\"5\" y=\"6\" width=\"14\" height=\"12\" rx=\"2\" fill=\"#58a6ff\"/></symbol></svg>"
        #expect(PixelFigure.drawn(sprites: sheet, scene: "") == ["s-needs": figure])
    }

    @Test internal func theBackdropIsItsFourLayersAndTheNightLook() {
        var drawing = Drawing()
        drawing.apply(.open("sea"))
        drawing.apply(.touch(col: 0, row: 99, began: true))
        drawing.apply(.touch(col: 131, row: 99, began: false))
        let scene = PixelFigure.scene(drawing.figures, Self.tokens)
        #expect(scene.contains("#night:target{display:inline}") && scene.contains("viewBox=\"0 0 132 100\""))
        #expect(scene.contains("<g id=\"sea\"><rect x=\"0\" y=\"99\" width=\"132\" height=\"1\""))
        let read = PixelFigure.drawn(sprites: "", scene: scene)
        #expect(read.keys.sorted() == PixelFigure.layers.sorted() && read["sea"] == drawing.figures["sea"])
        #expect(PixelFigure.hex(12, Self.tokens) == nil && PixelFigure.hex(9, Self.tokens) == "#10161a")
    }

    @Test internal func aDraftTakesTheDrawnStatesAsSymbolsAndTheLayersAsItsScene() {
        var draft = ThemeDraft(from: "farm", label: "T", style: .modern, dark: Self.tokens, light: [:])
        let figure = drawing(.touch(col: 0, row: 0, began: true)).figure ?? PixelFigure(blank: "s-needs")
        draft.draw(["s-needs": figure])
        #expect(draft.sprites == ["s-needs": figure.symbol("s-needs", Self.tokens)] && draft.scene == nil)
        draft.draw(["sky": PixelFigure(blank: "sky")])
        #expect(
            draft.sprites.isEmpty && draft.scene == PixelFigure.scene(["sky": PixelFigure(blank: "sky")], Self.tokens))
    }
}
