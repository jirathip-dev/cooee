import ComposableArchitecture
@testable import CooeeCore
import CooeeFeatures
@testable import CooeeUI
import Foundation
import SwiftUI
import Testing

@Suite(.serialized)
internal struct ThemeFrames {
    private static let schemes = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("design-system/schemes")
    private static let server = Server(url: URL(string: "https://mini.local") ?? URL(filePath: "/"), token: "t")

    private static func scheme(_ name: String) -> String {
        (try? String(contentsOf: schemes.appendingPathComponent("\(name).json"), encoding: .utf8)) ?? "{}"
    }

    private static func decode<T: Decodable>(_ json: String) -> T? {
        try? JSONDecoder().decode(T.self, from: Data(json.utf8))
    }

    private static let failing: GateReport? = decode(
        #"""
        {"fails": ["Light colours: working is 2.6:1 on cards; the gate needs 3:1."],
         "marks": {"--a": {"what": "--a on --card", "value": 2.56, "min": 3, "fix": "#8a6a00", "fixed": 4.0},
                   "--r": {"what": "--r on the needs-you card", "value": 4.3, "min": 3},
                   "--g": {"what": "--g on --card", "value": 4.9, "min": 3},
                   "--fg": {"what": "--fg on the needs-you card", "value": 10.4, "min": 4.5},
                   "--mute": {"what": "--mute on the needs-you card", "value": 4.95, "min": 4.5}}}
        """#)

    private static let body = [
        "..c..........c..", ".cc..........cc.", ".c.c........c.c.", "..cc..w..w..cc..", "....c.k..k.c....",
        ".....cccccc.....", "...cccccccccc...", "..cccccccccccc..", ".cccccccccccccc.", "..cccccccccccc..",
        "..c.c.c..c.c.c..", ".c..c..c..c..c..",
    ]
    private static let raised = [".c............c.", ".cc..........cc.", "..cc........cc.."]
    private static let inks: [Character: Int] = ["c": 0, "w": 8, "k": 9, "a": 2]

    private static func crab(_ state: String, waving: [Bool]) -> PixelFigure {
        var figure = PixelFigure(blank: "s-\(state)")
        figure.frames = waving.map { wave in
            var cells = figure.frames[0]
            for (row, line) in ((wave ? raised : []) + body.dropFirst(wave ? 3 : 0)).enumerated() {
                for (col, ink) in line.enumerated() { cells[(row + 8) * 24 + col + 4] = inks[ink] }
            }
            if state == "needs" { for row in [0, 1, 2, 4] { (cells[row * 24 + 11], cells[row * 24 + 12]) = (2, 2) } }
            return cells
        }
        return figure
    }

    private static func looks(_ style: String) -> Looks {
        let json =
            #"{"themes": {"tidepool": {"label": "Tidepool", "scheme": {"dark": "farm", "light": "farm-light"}, "#
            + #""sprites": true, "style": "\#(style)", "scene": ["sky", "sea", "rocks", "sand"], "#
            + #""words": {"needs": "waving", "idle": "basking", "working": "scuttling"}}, "#
            + #""farm": {"label": "farm", "scheme": {"dark": "farm", "light": "farm-light"}, "sprites": true}}, "#
            + #""schemes": {"farm": \#(scheme("farm")), "farm-light": \#(scheme("farm-light")), "#
            + #""paper": \#(scheme("paper"))}}"#
        var looks: Looks = decode(json) ?? Looks()
        looks.styles = Style.served
        return looks
    }

    private static func state(_ style: String, colours: Bool) -> ThemeEditorFeature.State {
        var state = ThemeEditorFeature.State(
            server: server, looks: looks(style), theme: "tidepool", style: style, new: false)
        state.draft.light["--a"] = "#a88a2a"
        state.art = nil
        state.reports = ["dark": GateReport(), "light": failing ?? GateReport()]
        state.dark = false
        if colours {
            state.colours = false
            state.open = "--a"
        }
        return state
    }

    private static func frame(
        _ name: String, _ style: String, phone: Bool = true, _ view: @escaping @MainActor @Sendable () -> some View
    ) -> Frame {
        Frame(
            name: "theme-\(name)-\(style)", width: phone ? 402 : 1440, height: phone ? 874 : 900,
            palette: style == "scene" ? .farmLight : .farmDark, phone: phone, folder: "v4-theme-editor",
            view: { AnyView(view()) }
        )
    }

    @MainActor
    private static func store(_ style: String, colours: Bool) -> StoreOf<ThemeEditorFeature> {
        Store(initialState: state(style, colours: colours)) { EmptyReducer() }
    }

    @MainActor
    private static func draw(_ style: String, passing: Bool = false) -> StoreOf<ThemeEditorFeature> {
        var state = state(style, colours: false)
        state.dark = true
        if passing { state.reports["light"] = GateReport() }
        for figure in ["edit", "read", "run", "idle", "done", "merged", "fail"] {
            state.drawing.figures["s-\(figure)"] = crab(figure, waving: [figure == "done"])
        }
        state.drawing.figures["s-needs"] = crab("needs", waving: [false, true])
        state.drawing.apply(.open("s-needs"))
        state.drawing.apply(.frame(1))
        return Store(initialState: state) { EmptyReducer() }
    }

    @MainActor
    private static func menu(_ style: String, broken: Bool) -> StoreOf<MenuFeature> {
        let looks = looks(style)
        let settings = Settings(style: StylePick(style, kept: looks.style(style)), theme: "tidepool")
        var state = MenuFeature.State(server: server, settings: settings)
        state.looks = looks
        state.looks.broken = broken ? ["tidepool": "sprites.svg: no s-fail"] : nil
        state.saved = "Tidepool"
        return Store(initialState: state) { EmptyReducer() }
    }

    @MainActor
    private static func look(_ store: StoreOf<MenuFeature>) -> some View {
        VStack(alignment: .leading, spacing: 18) { LookPage(store: store) }.padding(16).padding(.top, 54)
            .frame(maxHeight: .infinity, alignment: .top)
    }

    internal static func editor(_ style: String) -> [Frame] {
        [
            frame("look", style) { look(menu(style, broken: false)) },
            frame("pack", style) { ThemeEditor(store: store(style, colours: false)) },
            frame("colours", style) { ThemeColours(store: store(style, colours: true)) },
            frame("draw", style) { ThemeDraw(store: draw(style)).padding(.top, 54) },
            frame("mac", style, phone: false) { ThemeMac(store: draw(style, passing: true), renames: false) },
        ]
    }

    internal static var all: [Frame] {
        editor("modern") + editor("scene") + [frame("look-broken", "modern") { look(menu("modern", broken: true)) }]
    }

    @MainActor @Test(arguments: all)
    internal func matchesTheDesign(_ frame: Frame) throws { try frame.matchesItsDesign() }
}
