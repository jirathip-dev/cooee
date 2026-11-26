import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

// SAFETY: a JSON literal that decodes.
private let kitchen = try! served(
    JSONDecoder().decode(
        Looks.self,
        from: Data(
            #"{"themes":{"kitchen":{"label":"kitchen","scheme":{"dark":"k","light":"kl"},"sprites":true}},"schemes":{}}"#
                .utf8)
    ))
// SAFETY: any JSON object decodes as a ServedStyle; this one's style is nil, a format the app doesn't read.
private let newer = try! JSONDecoder().decode(ServedStyle.self, from: Data(#"{"format": 2}"#.utf8))
private let terminal = StylePick("terminal", kept: kitchen.style("terminal"))
private let scene = StylePick("scene", kept: kitchen.style("scene"))

@MainActor
internal struct MenuFeatureTests {
    nonisolated private static let menu = JSONValue(
        json: #"{"type": "screen", "id": "menu", "source": "core", "blocks": []}"#)
    private let calls = Calls()

    private func store(
        _ server: Server? = mini, style: StylePick = .modern, looks: Looks = Looks()
    ) -> TestStoreOf<MenuFeature> {
        let calls = calls
        var state = MenuFeature.State(server: server, settings: Settings(style: style))
        state.looks = looks
        return TestStore(initialState: state) {
            MenuFeature()
        } withDependencies: {
            $0.settingsStore.save = { settings in
                calls.record("save \(settings.scheme) \(settings.style.name) \(settings.theme) \(settings.push ?? "-")")
            }
            $0.threads.looks = { _ in kitchen }
            $0.appIcon.set = { icon in calls.record("icon \(icon.asset)") }
            $0.plugins.screen = { _, source, path in
                calls.record("screen \(source.name)\(path)")
                return Self.menu
            }
        }
    }

    @Test internal func theLookIsPickedAndKept() async {
        let store = store()
        await store.send(.open(true)) { $0.open = true }
        await store.receive(\.looks) { $0.looks = kitchen }
        await store.receive(\.loaded) { $0.screen = Self.menu }
        await store.send(.path([.look])) { $0.path = [.look] }
        await store.receive(\.looks)
        await store.send(.path([.look, .appIcon])) { $0.path = [.look, .appIcon] }
        await store.send(.scheme(.dark)) { $0.settings.scheme = .dark }
        await store.send(.style("terminal")) { $0.settings.style = terminal }
        await store.send(.theme("kitchen")) { $0.settings.theme = "kitchen" }
        await store.send(.open(false)) {
            $0.open = false
            $0.path = []
        }
        #expect(
            calls.all.filter { $0.hasPrefix("save") } == [
                "save dark modern farm -", "save dark terminal farm -", "save dark terminal kitchen -",
            ])
        #expect(calls.all.contains("icon TerminalPrompt") && calls.all.first == "screen core/screens/menu")
    }

    @Test internal func aServerRowOpensCoresScreenAndClosingTheMenuKeepsAndDropsIt() async {
        let (store, calls) = (store(), calls)
        store.dependencies.date = .constant(epoch)
        store.dependencies.continuousClock = TestClock()
        store.dependencies.screenCache.save = { key, kept in calls.record("keep \(key) \(kept.kept == epoch)") }
        await store.send(.show("/screens/settings/devices", label: "Devices")) {
            $0.page = ScreenFeature.State(
                server: mini, source: .core, path: "/screens/settings/devices", label: "Devices")
        }
        await store.send(.page(.presented(.loaded(Self.menu)))) {
            $0.page?.screen = LiveScreen(Self.menu)
            $0.page?.asOf = epoch
        }
        await store.send(.open(false)) { $0.page = nil }
        await store.finish()
        #expect(calls.all == ["keep https://mini.local|core|/screens/settings/devices true"])
        await store.send(.show("/screens/queue", label: "Queue", source: .plugin("drover"))) {
            $0.page = ScreenFeature.State(
                server: mini, source: .plugin("drover"), path: "/screens/queue", label: "Queue")
        }
    }

    @Test internal func offlineTheMenuKeepsTheBundledOneAndWithoutAServerOpensNothing() async {
        let store = store()
        store.dependencies.plugins.screen = { _, _, _ in throw APIError(status: 0, text: "offline") }
        await store.send(.open(true)) { $0.open = true }
        await store.receive(\.looks) { $0.looks = kitchen }
        let alone = self.store(nil)
        await alone.send(.open(true)) { $0.open = true }
        await alone.send(.show("/screens/settings/devices", label: "Devices"))
    }

    @Test internal func theLooksLoadOnlyWithAServerAndKeepFarmWhenTheReadFails() async {
        await store(nil).send(.load)
        let store = store()
        store.dependencies.threads.looks = { _ in throw APIError(status: 500, text: "") }
        await store.send(.load)
        #expect(store.state.looks == Looks() && store.state.settings.theme == "farm")
    }

    @Test internal func aPickedIconStaysThroughStyleChangesUntilTheLookIsFollowedAgain() async {
        let store = store(looks: kitchen)
        let koala = IconSet.bush.icons[2]
        await store.send(.icon(koala)) { $0.settings.icon = koala }
        await store.send(.style("scene")) { $0.settings.style = scene }
        await store.send(.icon(nil)) { $0.settings.icon = nil }
        await store.send(.icon(IconSet.farm.icons[0])) {
            $0.settings.icon = IconSet.farm.icons[0]
        }
        #expect(
            calls.all == [
                "save system modern farm -", "icon BushKoala", "save system scene farm -", "save system scene farm -",
                "icon FarmBay",
                "save system scene farm -",
            ])
    }

    @Test internal func theMotionSwitchTheNightPickAndAPickedSpriteAreKept() async {
        let store = store()
        await store.send(.motion(false)) { $0.settings.motion = false }
        #expect(!store.state.settings.moves(reduceMotion: false))
        await store.send(.sprite("coder", "blk")) { $0.settings.sprites = ["farm": ["coder": "blk"]] }
        await store.send(.sprite("coder", nil)) { $0.settings.sprites = ["farm": [:]] }
        await store.send(.night(true)) { $0.settings.night = true }
        #expect(calls.all == Array(repeating: "save system modern farm -", count: 4))
    }

    @Test internal func aRefusedIconChangeSaysWhichTileToTapAgain() async {
        let store = store(looks: kitchen)
        store.dependencies.appIcon.set = { _ in throw APIError(status: 0, text: "no") }
        await store.send(.style("terminal")) { $0.settings.style = terminal }
        await store.receive(\.failed) { $0.error = "Couldn't change the Home Screen icon. Tap Prompt to try again." }
    }
}

extension MenuFeatureTests {
    private static func answer(_ styles: [String: ServedStyle], broken: [String: String]? = nil) -> Looks {
        var looks = kitchen
        (looks.styles, looks.broken) = (styles, broken)
        return looks
    }

    @Test internal func aLaunchDrawsTheKeptTokensOrModernAndAnUpgradedPickSaysItIsAPackNow() {
        #expect(store().state.style(for: "farm") == nil && store().state.notice == nil)
        #expect(store(style: terminal).state.style(for: "farm") == terminal.kept && terminal.kept != nil)
        let upgraded = store(style: .pack(name: "terminal", kept: nil)).state
        #expect(upgraded.style(for: "farm") == nil)
        #expect(upgraded.notice == "terminal is a pack now: cooee theme add terminal")
    }

    @Test internal func anAnswerWithThePacksStyleKeepsItsTokensAndClearsTheLine() async {
        let store = store(style: .pack(name: "terminal", kept: nil))
        await store.send(.looks(kitchen)) {
            $0.looks = kitchen
            $0.settings.style = terminal
            $0.notice = nil
        }
        await store.finish()
        #expect(calls.all == ["save system terminal farm -", "icon TerminalPrompt"])
    }

    @Test(arguments: [
        (["terminal": newer], nil, "terminal needs a newer app · last good style kept"),
        (
            [:], ["terminal": #"style.json: surface "blur": not one of glass dashed solid"#],
            #"terminal/style.json: surface "blur": not one of glass dashed solid · last good style kept"#
        ),
    ])
    internal func anUnreadableOrBrokenStyleKeepsTheLastGoodTokens(
        _ styles: [String: ServedStyle], _ broken: [String: String]?, _ line: String
    ) async {
        let (store, looks) = (store(style: terminal), Self.answer(styles, broken: broken))
        await store.send(.looks(looks)) {
            $0.looks = looks
            $0.notice = line
        }
        #expect(store.state.style(for: "farm") == terminal.kept && calls.all.isEmpty)
    }

    @Test internal func aPackTheServerNoLongerHasFallsBackToModernAndSaysSo() async {
        let (store, looks) = (store(style: terminal), Self.answer([:]))
        await store.send(.looks(looks)) {
            $0.looks = looks
            $0.settings.style = .modern
            $0.notice = "Terminal is not installed on mini.local"
        }
        await store.finish()
        #expect(store.state.style(for: "farm") == nil)
        #expect(calls.all == ["save system modern farm -", "icon ModernLaughing"])
    }

    @Test internal func offlineTheKeptTokensDrawAndNothingIsShown() async {
        let store = store(style: terminal)
        store.dependencies.threads.looks = { _ in throw APIError(status: 0, text: "offline") }
        await store.send(.load)
        #expect(store.state.style(for: "farm") == terminal.kept && store.state.notice == nil)
    }

    @Test internal func aPackWithSpritesDrawsTheSceneAndOneWithoutDrawsModernButKeepsThePick() async throws {
        var looks = kitchen
        looks.themes["plain"] = try JSONDecoder().decode(
            ThemePack.self,
            from: Data(#"{"label": "plain", "scheme": {"dark": "p", "light": "pl"}, "sprites": false}"#.utf8))
        let store = store(style: scene)
        await store.send(.looks(looks)) { $0.looks = looks }
        #expect(store.state.offered.map(\.name) == ["modern", "scene", "terminal"])
        #expect(store.state.style(for: "kitchen")?.figures == .sprite)
        await store.send(.theme("plain")) { $0.settings.theme = "plain" }
        #expect(store.state.style(for: "plain") == nil && store.state.settings.style == scene)
        #expect(store.state.offered.map(\.name) == ["modern", "terminal"])
    }

    @Test internal func aPickedRowTakesThePacksTokensOrModern() async {
        let store = store()
        await store.send(.looks(kitchen)) { $0.looks = kitchen }
        #expect(store.state.offered.map(\.label) == ["Modern", "Scene", "Terminal"])
        await store.send(.style("scene")) { $0.settings.style = scene }
        await store.finish()
        #expect(store.state.style(for: "farm")?.figures == .sprite)
        await store.send(.style("modern")) { $0.settings.style = .modern }
        await store.finish()
        #expect(
            calls.all == [
                "save system scene farm -", "icon FarmBay", "save system modern farm -", "icon ModernLaughing",
            ])
    }
}
