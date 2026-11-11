import CooeeCore
import SwiftUI

public struct ChatActions {
    public var draft: Binding<String>?
    public var files: [String]
    public var send: (() -> Void)?
    public var attach: ([Attachment]) -> Void
    public var detach: (Int) -> Void
    public var stop: () -> Void
    public var steer: (Int) -> Void
    public var unqueue: (Int) -> Void
    public var answer: (Int, BlockAction) -> Void
    public var answering: Set<Int>
    public var back: () -> Void
    public var retry: (Int) -> Void = { _ in }
    public var places: [String] = []
    public var open: (Int) -> Void = { _ in }
    public var attachable: [String] = []
    public var attachPlugin: (Int) -> Void = { _ in }
    public var workflow: Int?
    public var showWorkflow: () -> Void = {}

    public init(
        draft: Binding<String>? = nil,
        files: [String] = [],
        send: (() -> Void)? = nil,
        attach: @escaping ([Attachment]) -> Void = { _ in },
        detach: @escaping (Int) -> Void = { _ in },
        stop: @escaping () -> Void = {},
        steer: @escaping (Int) -> Void = { _ in },
        unqueue: @escaping (Int) -> Void = { _ in },
        answer: @escaping (Int, BlockAction) -> Void = { _, _ in },
        answering: Set<Int> = [],
        back: @escaping () -> Void = {}
    ) {
        self.draft = draft
        self.files = files
        self.send = send
        self.attach = attach
        self.detach = detach
        self.stop = stop
        self.steer = steer
        self.unqueue = unqueue
        self.answer = answer
        self.answering = answering
        self.back = back
    }
}

public struct ChatView: View {
    @Environment(\.look) private var look
    private let persona: Persona
    private let items: [Item]
    private var working = false
    private var since: Date?
    private var queued: [String] = []
    private var draft = ""
    private var back = true
    private var chips: [String]
    private var actions = ChatActions()

    public var body: some View {
        ZStack {
            Backdrop()
            VStack(spacing: 0) {
                ChatHeader(persona: persona, chips: chips, back: back, actions: actions)
                Scroller(end: true) {
                    VStack(alignment: .leading, spacing: 14) {
                        ForEach(items) { item in
                            ItemView(item: item) { actions.retry(item.id) }.environment(\.act) {
                                actions.answer(item.id, $0)
                            }
                            .disabled(actions.answering.contains(item.id))
                        }
                    }
                    .padding(.horizontal, 16).padding(.top, 8).padding(.bottom, 16)
                }
                .frame(maxHeight: .infinity, alignment: .top)
                VStack(spacing: 8) {
                    if working { WorkingPill(since: since, stop: actions.stop) }
                    Composer(
                        persona: persona,
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
        queued: [String],
        back: Bool,
        actions: ChatActions
    ) {
        self.persona = persona
        self.items = items
        self.working = working
        self.since = since
        self.queued = queued
        self.back = back
        self.actions = actions
        chips = actions.places
    }

    public static func streaming(back: Bool = true) -> ChatView {
        ChatView(personaID: "coder", items: Fixtures.streaming, working: true, queued: Fixtures.queued, back: back)
    }

    public static func cards() -> ChatView { ChatView(personaID: "res", items: Fixtures.cards) }

    public static func conductor() -> ChatView { ChatView(personaID: "cond", items: Fixtures.conductor) }

    public static func moreCards() -> ChatView { ChatView(personaID: "res", items: Fixtures.moreCards) }
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
                if back {
                    Button(action: actions.back) { RoundButton(symbol: "chevron.left", ascii: "<", size: 40) }
                        .buttonStyle(.plain).accessibilityLabel("Back")
                }
                Avatar(persona: persona, size: 38)
                VStack(alignment: .leading, spacing: 3) {
                    Text(persona.name).ui(16, .semibold, look)
                    AgentChip(persona: persona)
                }
                Spacer(minLength: 4)
                more
            }
            if !chips.isEmpty || actions.workflow != nil { plugins }
        }
        .padding(.horizontal, 14).padding(.vertical, 8)
        .overlay(alignment: .bottom) { Rectangle().fill(palette.line).frame(height: 0.5) }
    }

    @ViewBuilder private var more: some View {
        if actions.attachable.isEmpty {
            RoundButton(symbol: "ellipsis", ascii: "…", size: 40)
        } else {
            Menu {
                ForEach(Array(actions.attachable.enumerated()), id: \.offset) { index, name in
                    Button("Attach \(name)…") { actions.attachPlugin(index) }
                }
            } label: {
                RoundButton(symbol: "ellipsis", ascii: "…", size: 40)
            }
            .buttonStyle(.plain).accessibilityLabel("More")
        }
    }

    private var plugins: some View {
        HStack(spacing: 8) {
            ForEach(Array(chips.enumerated()), id: \.offset) { index, plugin in
                Button {
                    actions.open(index)
                } label: {
                    Text(look.mono ? "[\(plugin.lowercased())]" : plugin).ui(13, .semibold, look)
                        .foregroundStyle(look.palette.green)
                        .padding(.horizontal, 12).frame(height: 30)
                        .modifier(RoundSurface(look: look))
                }
                .buttonStyle(.plain)
            }
            if let count = actions.workflow { WorkflowChip(count: count, open: actions.showWorkflow) }
        }
        .padding(.leading, back ? 50 : 0)
    }
}

internal struct WorkingPill: View {
    @Environment(\.look) private var look
    internal var since: Date?
    internal var stop: () -> Void = {}

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 8) {
            if look.mono {
                Text("~o=").ui(13, .bold, look).foregroundStyle(palette.cyan)
            } else {
                Circle().fill(palette.cyan).frame(width: 8, height: 8)
            }
            if let since {
                TimelineView(.periodic(from: since, by: 1)) { context in
                    label(Int(context.date.timeIntervalSince(since)))
                }
            } else {
                label(42)
            }
            Spacer()
            Button(action: stop) {
                HStack(spacing: 4) {
                    Image(systemName: "stop.fill").font(.system(size: 10))
                    Text("Stop")
                }
                .ui(13, .semibold, look).foregroundStyle(palette.red)
                .padding(.horizontal, 10).frame(height: 28).background(palette.red.opacity(0.14), in: .capsule)
            }
            .buttonStyle(.plain).accessibilityLabel("Stop")
        }
        .padding(.leading, 14).padding(.trailing, 6).frame(height: 40)
        .surface(look, radius: 20)
    }

    private func label(_ seconds: Int) -> some View {
        let time = "\(max(0, seconds) / 60):\(String(format: "%02d", max(0, seconds) % 60))"
        return Text(look.mono ? "working \(time)" : "Working · \(time)").ui(13, .medium, look)
    }
}
