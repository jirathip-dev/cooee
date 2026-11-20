import CooeeCore
import CooeeFeatures
@testable import CooeeUI
import SwiftUI
import Testing

@MainActor
private enum VoiceScreens {
    private static func persona(_ id: String, _ state: LaneState) -> Persona {
        let persona = Fixtures.persona(id)
        return Persona(api: persona.api, look: persona.look, state: state)
    }

    private static let said = "Rerun the export with the Q3"
    private static let unsure = Unsure(word: "ten Q", alternatives: ["10-Q", "10Q", "tenkey"])

    internal static func dictation(listening: Bool) -> some View {
        let heard = Heard(settled: said, pending: "ten Q filing", unsure: [unsure])
        var actions = ChatActions()
        actions.voice = VoiceActs(dictation: Dictation(before: "", heard: heard, listening: listening))
        let items: [Item] = [
            .you(id: 1, text: "Export Kestrel's filings to CSV, one row a line item."),
            .tools(id: 2, rows: [ToolRow(.run, "cargo run -p export", "ok"), ToolRow(.edit, "kestrel-q2.csv", "ok")]),
            .agent(id: 3, markdown: [.text("Done: **388 rows** in `kestrel-q2.csv`. The Q3 filing isn't in yet.")]),
        ]
        let draft = listening ? "" : "\(said) ten Q filing"
        return thread(persona("coder", .done), items) {
            Composer(hint: "Message Coder", draft: draft, actions: actions)
        }
    }

    internal static func call() -> some View {
        let lines = [
            Call.Line(you: true, text: "What did the Q3 10-Q say about Birch?"),
            Call.Line(you: false, text: "Birch is 14% of revenue, down from 19% in Q2."),
            Call.Line(you: true, text: "Why the drop?"),
        ]
        return CallScreen(
            persona: persona("res", .working), title: "Kestrel filings", seconds: 192, lines: lines,
            speaking: "The Birch contract renews in January, so it's deferred.", tool: "fetch sec.gov … notes, p. 14")
    }

    internal static func callThread() -> some View {
        let researcher = persona("res", .done)
        let lines = [
            Call.Line(you: true, text: "What did the Q3 10-Q say about Birch?"),
            Call.Line(you: false, text: "Birch is 14% of revenue, down from 19% in Q2."),
            Call.Line(you: true, text: "Why the drop?"),
            Call.Line(you: false, text: "Its contract renews in January, so it's deferred."),
            Call.Line(you: true, text: "Save that to the notes."),
            Call.Line(you: false, text: "Saved."),
        ]
        let items: [Item] = [
            .you(id: 1, text: "Find what the Q3 10-Q says about Birch."),
            .call(Call(id: 2, seconds: 252, lines: lines, tools: 2)),
            .tools(
                id: 3,
                rows: [ToolRow(.fetch, "sec.gov/…/kestrel-10q-q3.htm", "ok"), ToolRow(.read, "notes, p. 14", "ok")]),
            .agent(
                id: 4,
                markdown: [
                    .text("As I said on the call: Birch fell to **14%** because its renewal is deferred to January."),
                    .bullets(["Source: Q3 10-Q, notes p. 14", "Saved to `notes/kestrel.md`"]),
                ]),
        ]
        var actions = ChatActions()
        actions.voice.call = {}
        return thread(researcher, items, actions: actions) { Composer(hint: "Message Researcher") }
    }

    private static func thread(
        _ persona: Persona, _ items: [Item], actions: ChatActions = ChatActions(),
        @ViewBuilder composer: () -> some View
    ) -> some View {
        ZStack {
            Backdrop()
            VStack(spacing: 0) {
                ChatHeader(persona: persona, chips: [], actions: actions)
                VStack(alignment: .leading, spacing: 14) {
                    ForEach(items) { ItemView(item: $0) }
                }
                .padding(.horizontal, 16).padding(.top, 8)
                Spacer()
                composer().padding(.horizontal, 12).padding(.bottom, 26)
            }
            .environment(\.asker, persona)
        }
        .modifier(Inked())
    }
}

private struct Inked: ViewModifier {
    @Environment(\.look) private var look

    fileprivate func body(content: Content) -> some View {
        content.foregroundStyle(look.palette.foreground)
    }
}

@Suite(.serialized)
internal struct VoiceFrames {
    private static func frames(_ mode: String, _ palette: Palette) -> [Frame] {
        [
            Frame.phone("dictate-\(mode)", palette) { VoiceScreens.dictation(listening: true) },
            Frame.phone("dictate-edit-\(mode)", palette) { VoiceScreens.dictation(listening: false) },
            Frame.phone("call-\(mode)", palette) { VoiceScreens.call() },
            Frame.phone("call-thread-\(mode)", palette) { VoiceScreens.callThread() },
        ]
        .map { $0.filed("native-surfaces") }
    }

    internal static let all = frames("dark", .farmDark) + frames("light", .farmLight)
    internal static let packs = [
        (Style.terminal, Frame.phone("call-terminal-dark", .farmDark) { VoiceScreens.call() }),
        (Style.scene, Frame.phone("call-scene-light", .farmLight) { VoiceScreens.call() }),
    ]

    @MainActor @Test(arguments: all)
    internal func matchesTheDesign(_ frame: Frame) throws { try frame.matchesItsDesign() }

    @MainActor @Test(arguments: [0, 1])
    internal func matchesTheDesignInItsPack(_ index: Int) throws {
        let (style, frame) = Self.packs[index]
        try frame.filed("native-surfaces").matchesItsDesign(style)
    }
}
