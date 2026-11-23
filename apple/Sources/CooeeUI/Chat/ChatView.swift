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
                    .overlay(alignment: .bottom) { if loading { SlowStrip().offset(y: 2) } }
                }
                if let topics = actions.topics { StripRow(topics: topics, chips: chips, wide: !back, actions: actions) }
                Scroller(end: end, top: actions.older) {
                    Transcript {
                        if let topics = actions.topics {
                            TopicOpening(topics: topics, chips: back ? chips : [], actions: actions)
                        } else if items.isEmpty {
                            BigSprite(loading: loading)
                        }
                        ForEach(items) { item in
                            ItemView(
                                item: item, retry: { actions.retry(item.id) }, view: actions.view,
                                restart: actions.restart
                            ).equatable()
                                .environment(\.act) { actions.answer(item.id, $0) }
                                .disabled(actions.answering.contains(item.id))
                        }
                        LiveItem(reply: actions.live)
                    }
                    .padding(.horizontal, 16).padding(.top, actions.topics == nil ? 8 : 6).padding(.bottom, 16)
                }
                .frame(maxHeight: .infinity, alignment: .top)
                .environment(\.asker, persona)
                .environment(\.frames, actions.frames).environment(\.screenActs, actions.screenActs)
                .environment(\.message, actions.message)
                VStack(spacing: 8) {
                    if let waits = actions.waits { WaitsPill(name: persona.name, since: waits) }
                    if working { WorkingPill(persona: persona, since: since, step: actions.step, actions: actions) }
                    if let topics = actions.topics, topics.newing { FolderRow(actions: topics).padding(.bottom, 2) }
                    Composer(
                        hint: "Message \(actions.topics?.start?.title ?? persona.name)",
                        queued: queued,
                        draft: draft,
                        attachments: working && since == nil ? ["look-sheet.png"] : [],
                        actions: actions,
                        rest: 26
                    )
                }
                .padding(.horizontal, 12)
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

private struct Transcript<Content: View>: View {
    @Environment(\.look) private var look
    @ViewBuilder internal let content: Content

    internal var body: some View {
        if look.still {
            VStack(alignment: .leading, spacing: 14) { content }
        } else {
            LazyVStack(alignment: .leading, spacing: 14) { content }
        }
    }
}

private struct LiveItem: View {
    internal let reply: () -> LiveReply

    internal var body: some View {
        let live = reply()
        if !live.isEmpty { ItemView(item: .agent(id: Int.max, markdown: live.blocks)) }
    }
}
