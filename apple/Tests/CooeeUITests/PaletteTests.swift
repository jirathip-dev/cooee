import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import CooeeUI
import Foundation
import SwiftUI
import Testing

internal struct PaletteTests {
    private struct Scheme: Decodable {
        fileprivate let tokens: [String: String]
    }

    private static let schemes = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("schemes.json")

    private static func tokens(_ name: String) throws -> [String: String] {
        try #require(try JSONDecoder().decode([String: Scheme].self, from: Data(contentsOf: schemes))[name]).tokens
    }

    @Test internal func farmsSchemesAreTheDefaultPalettes() throws {
        #expect(Palette(tokens: try Self.tokens("farm"), dark: true) == .farmDark)
        #expect(Palette(tokens: try Self.tokens("farm-light"), dark: false) == .farmLight)
        #expect(Palette.farm(.dark) == .farmDark && Palette.farm(.light) == .farmLight)
    }

    @Test internal func aPacksSchemeIsItsPalette() throws {
        let kitchen = try #require(Palette(tokens: try Self.tokens("kitchen"), dark: true))
        #expect(kitchen != .farmDark && kitchen.dark)
        #expect(kitchen.background == Color(red: 0x1a / 255, green: 0x18 / 255, blue: 0x16 / 255))
        let garden = try #require(Palette(tokens: try Self.tokens("garden-light"), dark: false))
        #expect(garden.foreground == Color(red: 0x2a / 255, green: 0x2b / 255, blue: 0x22 / 255) && !garden.dark)
    }

    @Test internal func aSchemeMissingATokenIsNoPalette() throws {
        var tokens = try Self.tokens("kitchen")
        tokens["--fg"] = nil
        #expect(Palette(tokens: tokens, dark: true) == nil)
        tokens["--fg"] = "#abc"
        #expect(Palette(tokens: tokens, dark: true) == nil)
    }

    @MainActor @Test
    internal func pickingASchemeRepaintsEveryScreenInItsOwnModeAndIsKeptUntilTheServerDropsIt() async throws {
        let nord = try #require(
            try JSONDecoder().decode([String: ThemeScheme].self, from: Data(contentsOf: Self.schemes))["nord"])
        var state = MenuFeature.State(server: nil, settings: Settings(scheme: .light))
        state.looks.schemes["nord"] = nord
        let kept = LockIsolated<CooeeCore.Settings?>(nil)
        let store = TestStore(initialState: state) {
            MenuFeature()
        } withDependencies: {
            $0.settingsStore.save = { kept.setValue($0) }
        }
        #expect(store.state.palette(device: true) == .farmLight)
        await store.send(.colours("nord")) { $0.settings.colours = "nord" }
        #expect(store.state.palette(device: false) == Palette(tokens: nord.tokens, dark: true))
        #expect(store.state.dark(device: false))
        let relaunched = try JSONDecoder().decode(CooeeCore.Settings.self, from: JSONEncoder().encode(kept.value))
        #expect(relaunched.colours == "nord" && relaunched == store.state.settings)
        var dropped = store.state
        dropped.looks.schemes["nord"] = nil
        #expect(dropped.palette(device: false) == .farmLight)
        await store.send(.colours(nil)) { $0.settings.colours = nil }
        #expect(kept.value?.colours == nil && store.state.palette(device: true) == .farmLight)
    }
}
