import CooeeCore
import SwiftUI

public struct ChatView: View {
    @Environment(\.look) private var look
    private let persona: Persona
    private let items: [Item]
    private var working = false
    private var since: Date?
    private var loading = false
    private var queued: [String] = []
    private var draft = ""
    private var back = true
    private var headed = true
    private var chips: [String]
    private var actions = ChatActions()

    public var body: some View {
        ZStack {
            Backdrop()
            VStack(spacing: 0) {
                if headed {
                    ChatHeader(persona: persona, chips: chips, back: back, actions: actions)
                        .overlay(alignment: .bottom) { if loading { LoadingBar().offset(y: 2) } }
                }
                Scroller(end: true) {
                    VStack(alignment: .leading, spacing: 14) {
                        if items.isEmpty { BigSprite(loading: loading) }
                        ForEach(items) { item in
                            ItemView(item: item, retry: { actions.retry(item.id) }, view: actions.view)
                                .environment(\.act) { actions.answer(item.id, $0) }
                                .disabled(actions.answering.contains(item.id))
                        }
                    }
                    .padding(.horizontal, 16).padding(.top, 8).padding(.bottom, 16)
                }
                .frame(maxHeight: .infinity, alignment: .top)
                .environment(\.asker, persona)
                VStack(spacing: 8) {
                    if working { WorkingPill(persona: persona, since: since, step: actions.step, stop: actions.stop) }
                    Composer(
                        hint: "Message \(persona.name)",
                        queued: queued,
                        draft: draft,
                        attachments: working && since == nil ? ["look-sheet.png"] : [],
                        actions: actions
                    )
                }
                .padding(.horizontal, 12).padding(.bottom, 26)
            }
        }
        .foregroundStyle(look.palette.foreground)
    }

    public init(
        personaID: String,
        items: [Item],
        working: Bool = false,
        queued: [String] = [],
        draft: String = "",
        back: Bool = true
    ) {
        persona = Fixtures.persona(personaID)
        chips = persona.api.plugins
        self.items = items
        self.working = working
        self.queued = queued
        self.draft = draft
        self.back = back
    }

    public init(
        persona: Persona,
        items: [Item],
        working: Bool,
        since: Date,
        loading: Bool = false,
        queued: [String],
        back: Bool,
        headed: Bool = true,
        actions: ChatActions
    ) {
        self.persona = persona
        self.items = items
        self.working = working
        self.since = since
        self.loading = loading
        self.queued = queued
        self.back = back
        self.headed = headed
        self.actions = actions
        chips = actions.places
    }

    public static func streaming(back: Bool = true) -> ChatView {
        ChatView(personaID: "coder", items: Fixtures.streaming, working: true, queued: Fixtures.queued, back: back)
    }

    public static func cards() -> ChatView { ChatView(personaID: "res", items: Fixtures.cards) }

    public static func conductor() -> ChatView { ChatView(personaID: "cond", items: Fixtures.conductor) }

    public static func moreCards() -> ChatView { ChatView(personaID: "res", items: Fixtures.moreCards) }

    public static func asked() -> ChatView { ChatView(personaID: "coder", items: Fixtures.asked) }
}

internal struct ChatHeader: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal let chips: [String]
    internal var back = true
    internal var actions = ChatActions()

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 10) {
                if let title = actions.backTo {
                    BackLink(title: title, run: actions.back).padding(.horizontal, -14)
                } else if back {
                    Button(action: actions.back) { RoundButton(symbol: "chevron.left", ascii: "<", size: 40) }
                        .buttonStyle(.plain).accessibilityLabel("Back")
                }
                sheet { FigureView(persona: persona, size: 38) }
                VStack(alignment: .leading, spacing: 3) {
                    sheet { Text(persona.name).ui(16, .semibold, look) }
                    HStack(spacing: 6) {
                        chip
                        if let machine = actions.machine {
                            Button(action: actions.showMachine) { MachineChipLabel(machine: machine) }
                                .buttonStyle(.plain).accessibilityLabel("Machine \(machine.title)")
                        }
                    }
                }
                Spacer(minLength: 4)
                if let new = actions.new {
                    Button(action: new) { RoundButton(symbol: "plus", ascii: "+", size: 40) }
                        .buttonStyle(.plain).accessibilityLabel("New chat")
                }
                more
            }
            if !chips.isEmpty || actions.workflow != nil { plugins }
        }
        .padding(.horizontal, 14).padding(.vertical, 8)
        .overlay(alignment: .bottom) { Rectangle().fill(palette.line).frame(height: 0.5) }
    }

    private func sheet(_ label: () -> some View) -> some View {
        Button {
            actions.persona?()
        } label: {
            label()
        }
        .buttonStyle(.plain).accessibilityLabel(persona.name)
    }

    @ViewBuilder private var chip: some View {
        if let choices = actions.choices {
            AgentMenu(persona: persona, choices: choices, effort: false, pick: actions.agent, set: actions.model)
        } else {
            AgentChip(persona: persona)
        }
    }

    private var more: some View {
        Button(action: actions.more) { RoundButton(symbol: "ellipsis", ascii: "…", size: 40) }
            .buttonStyle(.plain).accessibilityLabel("More")
    }

    private var plugins: some View {
        HStack(spacing: 8) {
            ForEach(Array(chips.enumerated()), id: \.offset) { index, plugin in
                Button {
                    actions.open(index)
                } label: {
                    HStack(spacing: 6) {
                        Text(look.mono ? "[\(plugin.lowercased())]" : plugin)
                        if let count = actions.counts.indices.contains(index) ? actions.counts[index] : nil {
                            Badge(count: count, tint: look.palette.red)
                        }
                    }
                    .ui(13, .semibold, look).foregroundStyle(look.palette.green)
                    .padding(.horizontal, 12).frame(height: 30)
                    .modifier(RoundSurface(look: look))
                }
                .buttonStyle(.plain)
            }
            if let count = actions.workflow {
                WorkflowChip(count: count, lit: actions.workflowOpen, open: actions.showWorkflow)
            }
        }
        .padding(.leading, back ? 50 : 0)
    }
}
