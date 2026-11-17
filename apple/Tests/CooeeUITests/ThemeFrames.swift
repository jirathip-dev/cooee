import ComposableArchitecture
@testable import CooeeCore
import CooeeFeatures
@testable import CooeeUI
import Foundation
import SwiftUI

internal enum ThemeFrames {
    private static let schemes = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("design-system/schemes")
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
         "marks": {"--a": {"what": "--a on --card", "value": 2.56, "min": 3, "fix": "#8a6a00"},
                   "--r": {"what": "--r on the needs-you card", "value": 4.3, "min": 3},
                   "--g": {"what": "--g on --card", "value": 4.9, "min": 3},
                   "--fg": {"what": "--fg on the needs-you card", "value": 10.4, "min": 4.5},
                   "--mute": {"what": "--mute on the needs-you card", "value": 4.95, "min": 4.5}}}
        """#)

    private static func state(_ style: LookStyle, colours: Bool) -> ThemeEditorFeature.State {
        let json =
            #"{"themes": {"tidepool": {"label": "Tidepool", "scheme": {"dark": "farm", "light": "farm-light"}, "#
            + #""sprites": true, "style": "\#(style.rawValue)"}}, "schemes": {"farm": \#(scheme("farm")), "#
            + #""farm-light": \#(scheme("farm-light")), "paper": \#(scheme("paper"))}}"#
        var state = ThemeEditorFeature.State(
            server: server, looks: decode(json) ?? Looks(), theme: "tidepool", style: style, new: false)
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
        _ name: String, _ style: LookStyle, _ view: @escaping @MainActor @Sendable () -> some View
    )
        -> Frame
    {
        let light = style == .scene
        return Frame(
            name: "theme-\(name)-\(style.rawValue)", width: 402, height: 874,
            palette: light ? .farmLight : .farmDark, style: light ? .modern : style, phone: true,
            folder: "v4-theme-editor", view: { AnyView(view()) }
        )
    }

    @MainActor
    private static func store(_ style: LookStyle, colours: Bool) -> StoreOf<ThemeEditorFeature> {
        Store(initialState: state(style, colours: colours)) { EmptyReducer() }
    }

    internal static var all: [Frame] {
        LookStyle.allCases.flatMap { style in
            [
                frame("pack", style) { ThemeEditor(store: store(style, colours: false)) },
                frame("colours", style) { ThemeColours(store: store(style, colours: true)) },
            ]
        }
    }
}
