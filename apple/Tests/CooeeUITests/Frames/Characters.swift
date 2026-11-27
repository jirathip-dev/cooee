import CooeeCore
@testable import CooeeUI
import SwiftUI
import Testing

@Suite(.serialized)
internal struct CharacterFrames {
    private static let states: [LaneState] = [.idle, .working, .done, .needs, .fail]
    private static let looks: [String: (Palette, Style)] = [
        "modern-dark": (.farmDark, .bundled), "modern-light": (.farmLight, .bundled),
        "terminal-dark": (.farmDark, .terminal),
    ]

    private static func cast(_ palette: Palette, _ style: Style, size: CGFloat = 48) -> some View {
        Grid(horizontalSpacing: 12, verticalSpacing: 12) {
            ForEach(states, id: \.self) { state in
                GridRow {
                    ForEach(Mascot.allCases, id: \.self) { mascot in
                        FigureView(persona: Self.persona(mascot, state), size: size)
                    }
                }
            }
        }
        .padding(16).background(palette.background)
        .environment(\.look, Look(palette: palette, style: style, still: true))
        .environment(\.colorScheme, palette.dark ? .dark : .light)
    }

    private static func persona(_ mascot: Mascot, _ state: LaneState) -> Persona {
        var api = APIPersona(id: mascot.rawValue, name: mascot.name, agent: "")
        api.avatar = mascot.avatar
        return Persona(api, state: state)
    }

    private static func near(_ got: Double, _ want: Double) -> Bool { abs(got - want) < 0.001 }

    @MainActor @Test(arguments: looks.keys.sorted())
    internal func everyCharacterHoldsEachStatesKeyPose(_ look: String) throws {
        let (palette, style) = try #require(Self.looks[look])
        let renderer = ImageRenderer(content: Self.cast(palette, style))
        renderer.scale = 2
        try Frame.matches(try #require(renderer.cgImage), "characters-\(look)", in: "agent-birds/frames")
    }

    @Test internal func eachStateMovesTheWayTheDesignTimesIt() {
        #expect(Stance(.idle, at: 0).lid == 1 && Self.near(Stance(.idle, at: 0.93 * 4).lid, 0.08))
        #expect(Self.near(Stance(.idle, at: 1.6).lift, -1.2) && Stance(.idle, at: nil).lift == 0)
        #expect(Self.near(Stance(.working, at: 0).lean, -4) && Self.near(Stance(.working, at: 0.8).lean, 4))
        #expect(Self.near(Stance(.working, at: 0.3).dots[0], 1) && Self.near(Stance(.working, at: 0.3).dots[1], 0.25))
        #expect(Self.near(Stance(.working, at: 0.6).dots[1], 1) && Self.near(Stance(.working, at: 0.6).dots[2], 0.25))
        #expect(Self.near(Stance(.done, at: 0.42).lift, -12) && Self.near(Stance(.done, at: 1).lift, 0))
        #expect(Self.near(Stance(.done, at: nil).spark.shown, 1) && Stance(.idle, at: nil).spark.shown == 0)
        #expect(Self.near(Stance(.needs, at: 0.16).lean, -6) && Self.near(Stance(.needs, at: 0.16).pop, 1.25))
        #expect(Stance(.needs, at: 1.5).lean == 0 && Stance(.needs, at: nil).lid == 1.3)
        #expect(Stance(.fail, at: nil).tall == 0.93 && Self.near(Stance(.fail, at: 2).lift, 4))
    }
}
