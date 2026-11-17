import CooeeCore
@testable import CooeeUI
import SwiftUI
import Testing

@MainActor
internal struct TrustRenderTests {
    private static func pixels(_ view: some View) throws -> Pixels {
        let framed = view.frame(width: 402).environment(\.look, Look(palette: .farmDark, still: true))
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

    @Test(arguments: CardTrust.guarded.sorted())
    internal func theJSONCanNeitherSetNorFakeTheSourceLine(_ kind: String) throws {
        let missing = try Self.pixels(MissingPoster(kind: kind))
        #expect(Self.same(try Self.pixels(BlocksView(Self.card(kind, faking: true), source: nil)), missing))
        #expect(Self.same(try Self.pixels(BlocksView(Self.card(kind), source: nil)), missing))
        let core = try Self.pixels(BlocksView(Self.card(kind), source: .core))
        #expect(Self.same(try Self.pixels(BlocksView(Self.card(kind, faking: true), source: .core)), core))
        #expect(!Self.same(core, missing))
    }

    @Test internal func aPluginsPermissionsCardIsRefused() throws {
        let edgar = try Self.pixels(BlocksView(Self.card("permissions"), source: .plugin("edgar")))
        #expect(Self.same(edgar, try Self.pixels(MissingPoster(kind: "permissions"))))
        #expect(!Self.same(try Self.pixels(BlocksView(Self.card("secret"), source: .plugin("edgar"))), edgar))
    }
}
