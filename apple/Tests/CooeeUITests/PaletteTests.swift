import CooeeUI
import Foundation
import SwiftUI
import Testing

internal struct PaletteTests {
    private struct Scheme: Decodable {
        fileprivate let tokens: [String: String]
    }

    private static let schemes = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("web/schemes")

    private static func tokens(_ name: String) throws -> [String: String] {
        let data = try Data(contentsOf: schemes.appendingPathComponent("\(name).json"))
        return try JSONDecoder().decode(Scheme.self, from: data).tokens
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
}
