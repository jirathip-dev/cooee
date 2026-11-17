import ComposableArchitecture
@testable import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

// SAFETY: a JSON literal that decodes.
private let looks = try! JSONDecoder().decode(
    Looks.self,
    from: Data(
        #"""
        {"themes": {
          "farm": {"label": "farm", "scheme": {"dark": "farm", "light": "farm-light"}, "sprites": true},
          "tidepool": {"label": "Tidepool", "scheme": {"dark": "tp", "light": "tp-light"}, "sprites": true,
                       "style": "scene"}},
         "schemes": {
          "farm": {"label": "Farm", "color-scheme": "dark", "tokens": {"--bg": "#0f1a13", "--a": "#f29b4b"}},
          "farm-light": {"label": "Farm light", "color-scheme": "light", "tokens": {"--bg": "#f5efda"}},
          "tp": {"label": "Tidepool", "color-scheme": "dark", "tokens": {"--bg": "#10202a"}},
          "tp-light": {"label": "Tidepool light", "color-scheme": "light", "tokens": {"--a": "#a88a2a"}}}}
        """#.utf8)
)

// SAFETY: a JSON literal that decodes.
private let failing = try! JSONDecoder().decode(
    GateReport.self,
    from: Data(
        #"""
        {"fails": ["fails the contrast gate: --a on --card 2.56 (min 3)"],
         "marks": {"--a": {"what": "--a on --card", "value": 2.56, "min": 3, "fix": "#8a6a00"}}}
        """#.utf8)
)

private let idle =
    "<symbol id=\"s-idle\" viewBox=\"0 0 24 24\"><rect x=\"1\" y=\"2\" width=\"3\" height=\"1\" fill=\"#ffffff\"/></symbol>"
private let sheet = "<svg>\(idle)</svg>"

@MainActor
internal struct ThemeEditorFeatureTests {
    private let calls = Calls()

    private func store(_ theme: String, new: Bool = false) -> TestStoreOf<ThemeEditorFeature> {
        let calls = calls
        let state = ThemeEditorFeature.State(server: mini, looks: looks, theme: theme, style: .modern, new: new)
        return TestStore(initialState: state) {
            ThemeEditorFeature()
        } withDependencies: {
            $0.threads.checkTheme = { _, draft in
                calls.record("check \(draft.light["--a"] ?? "-")")
                return ["dark": GateReport(), "light": draft.light["--a"] == "#8a6a00" ? GateReport() : failing]
            }
            $0.threads.saveTheme = { _, name, draft in
                calls.record("save \(name) \(draft.label) \(draft.style) \(draft.sprites.keys.sorted())")
            }
            $0.threads.themeArt = { _, theme, file in
                calls.record("art \(theme)/\(file)")
                return file == "sprites.svg" ? sheet : ""
            }
        }
    }

    @Test internal func anOwnPackIsEditedInPlaceAndABuiltInIsCopied() {
        let own = ThemeEditorFeature.State(server: mini, looks: looks, theme: "tidepool", style: .modern, new: false)
        #expect(own.name == "tidepool" && own.draft.label == "Tidepool" && own.draft.style == .scene)
        #expect(own.draft.light == ["--a": "#a88a2a"] && own.draft.from == "tidepool")
        let copy = ThemeEditorFeature.State(server: mini, looks: looks, theme: "farm", style: .terminal, new: false)
        #expect(copy.name == "farm-copy" && copy.draft.label == "Farm copy" && copy.draft.style == .terminal)
        #expect(copy.art == nil && own.art?.absoluteString == "https://mini.local/look/tidepool/")
        let new = ThemeEditorFeature.State(server: mini, looks: looks, theme: "farm", style: .modern, new: true)
        #expect(new.draft.from == "starter" && new.name == "new-theme" && new.art == nil)
        #expect(new.draft.dark["--a"] == "#f29b4b")
    }

    @Test internal func saveWaitsForTheGateAndTheOneTapFixFreesIt() async {
        let store = store("tidepool")
        await store.send(.task)
        await store.receive(\.art) { $0.drawing.figures = PixelFigure.drawn(sprites: sheet, scene: "") }
        await store.receive(\.checked) { $0.reports = ["dark": GateReport(), "light": failing] }
        #expect(!store.state.canSave && store.state.fails == failing.fails)
        #expect(store.state.drawing.figures["s-idle"]?[0, 3, 2] == 8)
        await store.send(.save)
        await store.send(.colours(false)) { $0.colours = false }
        await store.send(.open("--a")) { $0.open = "--a" }
        await store.send(.token("--a", "#8a6a00")) { $0.draft.light["--a"] = "#8a6a00" }
        await store.receive(\.checked) { $0.reports = ["dark": GateReport(), "light": GateReport()] }
        await store.send(.colours(nil)) {
            $0.colours = nil
            $0.open = nil
        }
        #expect(store.state.canSave)
        await store.send(.save) {
            $0.draft.draw($0.drawing.figures)
            $0.saving = true
        }
        await store.receive(\.saved)
        let art = ["art tidepool/scene.svg", "art tidepool/sprites.svg", "check #8a6a00", "check #a88a2a"]
        #expect(calls.all.sorted() == art + ["save tidepool Tidepool scene [\"s-idle\"]"])
    }

    @Test internal func aDrawnFigureAndBackdropGoInTheSavedDraft() async {
        let store = store("farm", new: true)
        await store.send(.draw(.open("s-needs"))) { $0.drawing.apply(.open("s-needs")) }
        await store.send(.draw(.touch(col: 3, row: 4, began: true))) {
            $0.drawing.apply(.touch(col: 3, row: 4, began: true))
        }
        await store.send(.draw(.open("sky"))) { $0.drawing.apply(.open("sky")) }
        await store.send(.draw(.touch(col: 0, row: 0, began: true))) {
            $0.drawing.apply(.touch(col: 0, row: 0, began: true))
        }
        await store.send(.checked(["dark": GateReport(), "light": GateReport()])) {
            $0.reports = ["dark": GateReport(), "light": GateReport()]
        }
        await store.send(.save) {
            $0.draft.draw($0.drawing.figures)
            $0.saving = true
        }
        #expect(store.state.draft.sprites["s-needs"]?.contains("<rect x=\"3\" y=\"4\" width=\"1\"") == true)
        #expect(store.state.draft.scene?.contains("<g id=\"sky\"><rect x=\"0\" y=\"0\"") == true)
        await store.receive(\.saved)
        #expect(calls.all == ["save new-theme New theme modern [\"s-needs\"]"])
    }

    @Test internal func aStartingSchemeLabelStyleAndPreviewChangeTheDraft() async {
        let store = store("farm")
        await store.send(.token("--a", "#000000"))
        await store.send(.colours(true)) { $0.colours = true }
        await store.send(.start("tp")) { $0.draft.dark = ["--bg": "#10202a"] }
        await store.receive(\.checked) { $0.reports = ["dark": GateReport(), "light": failing] }
        await store.send(.label("Rock pool")) { $0.draft.label = "Rock pool" }
        await store.send(.style(.terminal)) { $0.draft.style = .terminal }
        await store.send(.preview(chat: true, dark: false)) {
            $0.chat = true
            $0.dark = false
        }
        #expect(store.state.name == "rock-pool" && !store.state.canSave)
    }

    @Test internal func aRefusedSaveShowsWhy() async {
        let store = TestStore(
            initialState: ThemeEditorFeature.State(
                server: mini, looks: looks, theme: "tidepool", style: .modern, new: false)
        ) {
            ThemeEditorFeature()
        } withDependencies: {
            $0.threads.checkTheme = { _, _ in throw APIErrorStub() }
            $0.threads.saveTheme = { _, _, _ in throw APIErrorStub() }
            $0.threads.themeArt = { _, _, _ in throw APIErrorStub() }
        }
        await store.send(.task)
        await store.receive(\.art)
        await store.receive(\.failed) { $0.error = APIErrorStub().localizedDescription }
        await store.send(.checked(["dark": GateReport(), "light": GateReport()])) {
            $0.reports = ["dark": GateReport(), "light": GateReport()]
        }
        await store.send(.save) {
            $0.saving = true
            $0.error = nil
        }
        await store.receive(\.failed) {
            $0.saving = false
            $0.error = APIErrorStub().localizedDescription
        }
    }
}

private struct APIErrorStub: Error {}

@MainActor
internal struct ThemeEditorMenuTests {
    private let calls = Calls()

    private func store() -> TestStoreOf<MenuFeature> {
        let calls = calls
        var state = MenuFeature.State(server: mini, settings: Settings())
        state.looks = looks
        return TestStore(initialState: state) {
            MenuFeature()
        } withDependencies: {
            $0.settingsStore.save = { calls.record("save \($0.theme) \($0.style)") }
            $0.threads.looks = { _ in looks }
            $0.threads.checkTheme = { _, _ in [:] }
            $0.threads.themeArt = { _, _, _ in "" }
        }
    }

    @Test internal func theMenuOpensTheEditorAndASavedPackIsAppliedWithNoRestart() async {
        let store = store()
        await store.send(.theme("tidepool")) {
            $0.settings.theme = "tidepool"
            $0.settings.style = .scene
        }
        await store.send(.edit("tidepool")) {
            $0.editor = ThemeEditorFeature.State(
                server: mini, looks: looks, theme: "tidepool", style: .scene, new: false)
            $0.path = [.themeEditor]
        }
        await store.receive(\.editor.task)
        await store.receive(\.editor.art)
        await store.receive(\.editor.checked)
        await store.send(.editor(.saved("tidepool", .terminal))) {
            $0.settings.style = .terminal
            $0.saved = "Tidepool"
            $0.editor = nil
            $0.path = []
        }
        await store.receive(\.looks)
        #expect(calls.all == ["save tidepool scene", "save tidepool terminal"])
    }
}
