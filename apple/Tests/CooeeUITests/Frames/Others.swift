import CooeeCore
@testable import CooeeUI
import Foundation
import SwiftUI
import Testing

@Suite(.serialized)
internal struct OthersFrames {
    internal static let themes: [String: [String: String]] = [
        "farm": Look.farmWords,
        "kitchen": ["working": "on the line", "needs": "the bell", "done": "at the pass", "idle": "standing by"],
        "garden": ["working": "being tended", "needs": "thirsty", "done": "ready", "idle": "resting"],
        "plain": [:],
    ]

    internal static func lane(_ item: String, _ state: LaneState, status: String? = nil) -> ListedChat {
        let label = WorkflowLabel(source: "drover", group: "cooee", item: item, role: "impl", round: "1")
        let name = "\(label.group) \(label.item) · \(label.stage)"
        let persona = Persona(
            api: APIPersona(id: "", name: name, agent: "drover"),
            look: PersonaLook(initials: "C", tint: .lead), state: state, seed: "t-\(item)")
        var row = Row(id: "t-\(item)", persona: "", title: name, last: "", ago: "4m", unseen: 0, state: state)
        row.status = status
        return ListedChat(row: row, persona: persona, workflow: label)
    }

    internal static let lanes = [
        lane("#839", .working), lane("#838", .needs), lane("#828", .idle), lane("#822", .done),
    ]

    @MainActor
    internal static func render(_ chats: [ListedChat], words: [String: String], style: Style = .bundled) -> CGImage? {
        var look = Look(palette: .farmDark, style: style, still: true)
        look.words = words
        let view = OthersView(chats: chats, notes: [], open: { _ in }, back: {}).frame(width: 390, height: 520)
            .background(look.palette.background).environment(\.look, look).environment(\.colorScheme, .dark)
        let renderer = ImageRenderer(content: view)
        renderer.scale = 2
        return renderer.cgImage
    }

    @MainActor
    @Test(arguments: [
        ("farm", "scene"), ("farm", "modern"), ("kitchen", "modern"), ("garden", "modern"), ("plain", "modern"),
    ])
    internal func matchesTheDesign(_ theme: String, _ style: String) throws {
        let words = try #require(Self.themes[theme])
        let image = try #require(Self.render(Self.lanes, words: words, style: style == "scene" ? .scene : .bundled))
        try Frame.matches(image, "others-\(theme)-\(style)-390", in: "others")
    }
}
