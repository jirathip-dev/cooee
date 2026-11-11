import ComposableArchitecture
import CooeeCore
import CooeeFeatures
@testable import CooeeUI
import SwiftUI
import Testing

@MainActor
internal struct MenuRenderTests {
    private static let palette = Palette.farmDark

    private static func pixels(_ view: some View) throws -> Pixels {
        let framed = view.frame(width: 402, height: 600).background(Color.red)
            .environment(\.look, Look(palette: palette, still: true))
        return try #require(Pixels(try #require(ImageRenderer(content: framed).cgImage)))
    }

    private static func pixel(_ pixels: Pixels, column: Int) -> ArraySlice<UInt8> {
        let start = (pixels.height / 2 * pixels.width + column) * 4
        return pixels.bytes[start..<start + 4]
    }

    @Test(arguments: MenuFeature.Page.allCases)
    internal func aPageFillsTheSheetWithItsBackground(_ page: MenuFeature.Page) throws {
        let store = Store(initialState: MenuFeature.State(server: nil, settings: Settings())) { MenuFeature() }
        let drawn = try Self.pixels(MenuPage(store: store, page: page))
        let sheet = try Self.pixels(Self.palette.background)
        for column in [0, drawn.width - 1] {
            #expect(Self.pixel(drawn, column: column) == Self.pixel(sheet, column: column), "\(page) column \(column)")
        }
    }
}
