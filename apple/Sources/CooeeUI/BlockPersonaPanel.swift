import CooeeCore
import SwiftUI

internal struct PanelActions {
    internal var close: () -> Void = {}
    internal var edit: (() -> Void)?
    internal var newChat: (() -> Void)?
    internal var topic = false
    internal var memory: (() -> Void)?
    internal var stop: (() -> Void)?
    internal var open: (Int) -> Void = { _ in }
    internal var workflow: () -> Void = {}
    internal var routine: (() -> Void)?
    internal var teach: ((String) -> Void)?
}

public struct PersonaPanelBlocks: View {
    @Environment(\.look) private var look
    private let persona: Persona
    private let screen: JSONValue
    private let column: Bool
    private let end: Bool
    private let actions: PanelActions

    public var body: some View {
        let palette = look.palette
        VStack(spacing: 0) {
            if !column {
                Capsule().fill(palette.mute.opacity(0.5)).frame(width: 36, height: 5).padding(.top, 6)
            }
            header
            Scroller(end: end) {
                PersonaBlocks(
                    screen: screen, open: actions.open, workflow: actions.workflow, routine: actions.routine,
                    teach: actions.teach
                ).padding(.horizontal, 16)
                    .padding(.top, 4).padding(
                        .bottom, 16)
            }
            footer.padding(.horizontal, 16).padding(.top, 10).padding(.bottom, column ? 16 : 30)
                .overlay(alignment: .top) { Rectangle().fill(palette.line).frame(height: 0.5) }
        }
        .background(background)
        .foregroundStyle(palette.foreground)
    }

    public init(personaID: String = "cond", column: Bool = false, end: Bool = false) {
        persona = Fixtures.persona(personaID)
        screen = JSONValue.personaPanels[personaID] ?? .bundledPersona
        self.column = column
        self.end = end
        let edit = screen.list("blocks").contains { $0.type == "edit" }
        actions = PanelActions(edit: edit ? {} : nil, newChat: {}, memory: {}, routine: {})
    }

    internal init(persona: Persona, screen: JSONValue, column: Bool, actions: PanelActions) {
        self.persona = persona
        self.screen = screen
        self.column = column
        self.actions = actions
        end = false
    }
}

extension PersonaPanelBlocks {
    private var header: some View {
        HStack(spacing: 10) {
            FigureView(persona: persona, size: column ? 36 : 44)
            VStack(alignment: .leading, spacing: 3) {
                Text(persona.name).ui(column ? 16 : 18, .semibold, look)
                AgentChip(persona: persona)
            }
            Spacer(minLength: 4)
            Button(action: actions.close) {
                if column {
                    RoundButton(symbol: "sidebar.right", ascii: "]", size: 34)
                } else {
                    RoundButton(symbol: "xmark", ascii: "x", size: 36)
                }
            }
            .buttonStyle(.plain).accessibilityLabel(column ? "Hide panel" : "Close")
        }
        .padding(.horizontal, 16).padding(.vertical, 10)
    }

    private var footer: some View {
        let blocks = screen.list("blocks")
        let count = Int(blocks.first { $0.type == "memory" }?["count"]?.number ?? 0)
        let memory = count > 0 ? "Memory · \(count)" : "Memory"
        return HStack(spacing: 10) {
            if let edit = actions.edit { action("pencil", "Edit", "Edit \(persona.name)", edit) }
            if let newChat = actions.newChat {
                let new = actions.topic ? "New topic" : "New chat"
                action("square.and.pencil", new, "\(new) with \(persona.name)", newChat)
            }
            if let open = actions.memory { action("brain.head.profile", memory, memory, open) }
            if let stop = actions.stop { action("stop.fill", "Stop", "Stop \(persona.name)", stop) }
        }
    }

    private func action(_ symbol: String, _ title: String, _ label: String, _ run: @escaping () -> Void) -> some View {
        let palette = look.palette
        return Button(action: run) {
            VStack(spacing: 6) {
                if look.mono {
                    Text("[\(title.lowercased())]").ui(13, .semibold, look)
                } else {
                    Image(systemName: symbol).font(.system(size: 17, weight: .semibold))
                    Text(title).ui(12, .medium, look).lineLimit(1)
                }
            }
            .foregroundStyle(palette.link).frame(maxWidth: .infinity).frame(height: 64)
            .background(
                palette.card.opacity(look.style.surface == .glass ? 0.75 : 1),
                in: .rect(cornerRadius: look.mono ? 0 : 16)
            )
            .overlay {
                if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) }
            }
        }
        .buttonStyle(.plain).accessibilityLabel(label)
    }

    @ViewBuilder private var background: some View {
        let palette = look.palette
        if column {
            palette.background.overlay(alignment: .leading) { Rectangle().fill(palette.line).frame(width: 1) }
        } else {
            UnevenRoundedRectangle(topLeadingRadius: 34, topTrailingRadius: 34).fill(palette.background)
                .overlay(
                    UnevenRoundedRectangle(topLeadingRadius: 34, topTrailingRadius: 34)
                        .fill(palette.card.opacity(0.35))
                )
                .ignoresSafeArea()
        }
    }
}

extension JSONValue {
    internal static let bundledPersona: JSONValue = {
        // SAFETY: Default/screens/persona.json is copied into this module's bundle by Package.swift
        let url = Bundle.module.url(forResource: "persona", withExtension: "json", subdirectory: "Default/screens")!
        return (try? JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: url))) ?? .null
    }()
}
