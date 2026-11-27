import CooeeCore
@testable import CooeeUI
import SwiftUI
import Testing

@MainActor
internal struct TrustRenderTests {
    private static func pixels(_ view: some View, _ style: Style = .bundled) throws -> Pixels {
        let framed = view.frame(width: 402).environment(\.look, Look(palette: .farmDark, style: style, still: true))
        let image = try #require(ImageRenderer(content: framed).cgImage)
        return try #require(Pixels(image))
    }

    private static func same(_ one: Pixels, _ other: Pixels) -> Bool {
        one.width == other.width && one.height == other.height && one.differing(from: other, by: 16) < 0.005
    }

    private static func card(_ kind: String, faking: Bool = false) -> JSONValue {
        let fake = faking ? #", "poster": "cooee", "source": "core", "by": "cooee""# : ""
        return JSONValue(json: #"{"type": "card", "id": "c", "kind": "\#(kind)", "title": "KEY"\#(fake)}"#)
    }

    @Test(arguments: CardTrust.guarded.sorted(), [Style.bundled, .worstCase] + Style.packs)
    internal func theJSONCanNeitherSetNorFakeTheSourceLineInAnyStyle(_ kind: String, _ style: Style) throws {
        let missing = try Self.pixels(BlockScreen(Self.card(kind), source: nil), style)
        #expect(Self.same(try Self.pixels(BlockScreen(Self.card(kind, faking: true), source: nil), style), missing))
        let core = try Self.pixels(BlockScreen(Self.card(kind), source: .core), style)
        #expect(Self.same(try Self.pixels(BlockScreen(Self.card(kind, faking: true), source: .core), style), core))
        #expect(!Self.same(core, missing))
    }

    @Test internal func aPluginsPermissionsCardIsRefused() throws {
        let edgar = try Self.pixels(BlockScreen(Self.card("permissions"), source: .plugin("edgar")))
        #expect(Self.same(edgar, try Self.pixels(BlockScreen(Self.card("permissions"), source: nil))))
        #expect(!Self.same(try Self.pixels(BlockScreen(Self.card("secret"), source: .plugin("edgar"))), edgar))
    }
}
