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
    internal var draft = ""
    private var back = true
    private var headed = true
    private var chips: [String]
    internal var end = true
    internal var actions = ChatActions()

    public var body: some View {
        ZStack {
            Backdrop()
            VStack(spacing: 0) {
                if headed {
                    ChatHeader(
                        persona: persona, chips: chips, back: back, plugged: actions.topics == nil, actions: actions
                    )
                    .overlay(alignment: .bottom) { if loading { LoadingBar().offset(y: 2) } }
                }
                if let topics = actions.topics { StripRow(topics: topics, chips: chips, wide: !back, actions: actions) }
                Scroller(end: end) {
                    VStack(alignment: .leading, spacing: 14) {
                        if let topics = actions.topics {
                            TopicOpening(topics: topics, chips: back ? chips : [], actions: actions)
                        } else if items.isEmpty {
                            BigSprite(loading: loading)
                        }
                        ForEach(items) { item in
                            ItemView(
                                item: item, retry: { actions.retry(item.id) }, view: actions.view,
                                restart: actions.restart
                            )
                            .environment(\.act) { actions.answer(item.id, $0) }
                            .disabled(actions.answering.contains(item.id))
                        }
                    }
                    .padding(.horizontal, 16).padding(.top, actions.topics == nil ? 8 : 6).padding(.bottom, 16)
                }
                .frame(maxHeight: .infinity, alignment: .top)
                .environment(\.asker, persona)
                .environment(\.frames, actions.frames).environment(\.screenActs, actions.screenActs)
                VStack(spacing: 8) {
                    if let waits = actions.waits { WaitsPill(name: persona.name, since: waits) }
                    if working { WorkingPill(persona: persona, since: since, step: actions.step, stop: actions.stop) }
                    if let topics = actions.topics, topics.newing { FolderRow(actions: topics).padding(.bottom, 2) }
                    Composer(
                        hint: "Message \(actions.topics?.start?.title ?? persona.name)",
                        queued: queued,
                        draft: draft,
                        attachments: working && since == nil ? ["look-sheet.png"] : [],
                        actions: actions
                    )
                }
                .padding(.horizontal, 12).padding(.bottom, 26)
            }
            .overlayPreferenceValue(PressedChip.self) { anchor in
                if let topics = actions.topics { TopicOverlays(topics: topics, anchor: anchor) }
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
    internal var plugged = true
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
                if let screen = actions.screen {
                    Button(action: screen) { RoundButton(symbol: "display", ascii: "scr", size: 40) }
                        .buttonStyle(.plain).accessibilityLabel("Screen")
                }
                more
            }
            if plugged && (!chips.isEmpty || actions.workflow != nil) { plugins }
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
        PluginChips(chips: chips, actions: actions).padding(.leading, back ? 50 : 0)
    }
}

internal struct PluginChips: View {
    @Environment(\.look) private var look
    internal let chips: [String]
    internal let actions: ChatActions

    internal var body: some View {
        HStack(spacing: 8) {
            ForEach(Array(chips.enumerated()), id: \.offset) { index, plugin in
                Button {
                    actions.open(index)
                } label: {
                    HStack(spacing: 6) {
                        Text(look.label(.chip, plugin))
                        if let count = actions.counts.indices.contains(index) ? actions.counts[index] : nil {
                            Badge(count: count, tint: look.palette.red)
                        }
                    }
                    .ui(13, .semibold, look).foregroundStyle(look.palette.green)
                    .padding(.horizontal, 12).frame(height: 30)
                    .chip(look)
                }
                .buttonStyle(.plain)
            }
            if let count = actions.workflow {
                WorkflowChip(count: count, lit: actions.workflowOpen, open: actions.showWorkflow)
            }
        }
    }
}
