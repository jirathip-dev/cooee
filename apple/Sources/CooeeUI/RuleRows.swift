import CooeeCore
import SwiftUI

extension RuleEffect {
    internal var title: String {
        switch self {
        case .allow: "Allow"
        case .ask: "Ask first"
        case .deny: "Deny"
        }
    }

    internal func tint(_ palette: Palette) -> Color {
        switch self {
        case .allow: palette.green
        case .ask: palette.amber
        case .deny: palette.red
        }
    }
}

internal struct EffectMark: View {
    @Environment(\.look) private var look
    internal let effect: RuleEffect
    internal let kind: ToolKind

    internal var body: some View {
        let tint = effect.tint(look.palette)
        if look.textMarks {
            Text("[\(effect.rawValue)]".padding(toLength: 7, withPad: " ", startingAt: 0)).ui(13, .bold, look)
                .foregroundStyle(tint).fixedSize()
        } else {
            Image(systemName: kind.symbol).font(.system(size: 14, weight: .semibold)).foregroundStyle(.white)
                .frame(width: 30, height: 30).background(tint, in: .rect(cornerRadius: look.radius(8)))
        }
    }
}

internal struct RuleLine: View {
    @Environment(\.look) private var look
    internal let rule: Rule
    internal var never: String?
    internal var hover = false
    internal var remove: () -> Void = {}

    internal var body: some View {
        let palette = look.palette
        HStack(alignment: .center, spacing: 12) {
            EffectMark(effect: rule.effect, kind: rule.kind)
            VStack(alignment: .leading, spacing: 3) {
                Text(rule.words).ui(15.5, .regular, look).fixedSize(horizontal: false, vertical: true)
                HStack(spacing: 0) {
                    Text(rule.origin).foregroundStyle(palette.mute)
                    if let never { Text(" · \(never) never asks for this").foregroundStyle(palette.amber) }
                }
                .ui(12.5, .regular, look).lineLimit(1)
            }
            Spacer(minLength: 4)
            if hover {
                Button(action: remove) { ActionPill(title: "Remove", tint: palette.red) }.buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 14).padding(.vertical, 10)
        .background(hover ? palette.link.opacity(0.10) : .clear)
        .overlay(alignment: .bottom) {
            Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, look.textMarks ? 0 : 56)
        }
    }
}

internal struct AddRuleRow: View {
    @Environment(\.look) private var look

    internal var body: some View {
        HStack(spacing: 10) {
            Glyph(symbol: "plus.circle.fill", ascii: "+", size: 16)
            Text(look.quiet ? "add a rule" : "Add a rule").ui(15.5, .regular, look)
            Spacer()
        }
        .foregroundStyle(look.palette.link).padding(.horizontal, 14).frame(height: 48).contentShape(Rectangle())
    }
}

internal struct RulesBody: View {
    @Environment(\.look) private var look
    internal let name: String
    internal let rules: [Rule]
    internal var agent: AgentOption?
    internal var fresh: String?
    @State internal var hover: String?
    @State internal var swiped: String?
    internal var add: () -> Void = {}
    internal var remove: (Rule) -> Void = { _ in }

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 22) {
            Group2(header: "\(name)'s rules") {
                ForEach(rules) { rule in
                    line(rule).background(rule.id == fresh ? palette.green.opacity(0.10) : .clear)
                }
                Button(action: add) { AddRuleRow() }.buttonStyle(.plain).accessibilityLabel("Add a rule")
            }
            Group2(header: "Anything else") {
                HStack {
                    Text("Ask you on a card").ui(15.5, .regular, look)
                    Spacer()
                    StateWord(text: "ask", tint: palette.amber)
                }
                .padding(.horizontal, 14).frame(height: 48)
            }
        }
    }

    private func never(_ rule: Rule) -> String? {
        guard let agent, agent.silent.contains(rule.kind) else { return nil }
        return agent.id
    }

    @ViewBuilder private func line(_ rule: Rule) -> some View {
        let row = RuleLine(rule: rule, never: never(rule), hover: rule.id == hover) { remove(rule) }
            .contentShape(Rectangle())
            .onHover { inside in hover = inside ? rule.id : (hover == rule.id ? nil : hover) }
            .gesture(
                DragGesture(minimumDistance: 20).onEnded { drag in
                    withAnimation(.snappy) { swiped = drag.translation.width < -40 ? rule.id : nil }
                })
        if rule.id == swiped {
            ZStack(alignment: .trailing) {
                look.palette.red
                Button {
                    swiped = nil
                    remove(rule)
                } label: {
                    Text(look.label(.button, "Remove")).ui(15, .semibold, look).foregroundStyle(.white)
                        .frame(width: 96).frame(maxHeight: .infinity)
                }
                .buttonStyle(.plain)
                row.background(look.palette.card).offset(x: -96)
            }
            .clipped()
        } else {
            row
        }
    }
}
