import CooeeCore
@testable import CooeeUI
import SwiftUI

internal struct EffectMark: View {
    @Environment(\.look) private var look
    internal let effect: Effect
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
    internal let rule: RuleFixture
    internal var agent = ""
    internal var divider = true
    internal var hover = false

    internal var body: some View {
        let palette = look.palette
        HStack(alignment: .center, spacing: 12) {
            EffectMark(effect: rule.effect, kind: rule.kind)
            VStack(alignment: .leading, spacing: 3) {
                Text(rule.words).ui(15.5, .regular, look).fixedSize(horizontal: false, vertical: true)
                HStack(spacing: 0) {
                    Text(rule.from).foregroundStyle(palette.mute)
                    if rule.silent { Text(" · \(agent) never asks for this").foregroundStyle(palette.amber) }
                }
                .ui(12.5, .regular, look).lineLimit(1)
            }
            Spacer(minLength: 4)
            if hover { ActionPill(title: "Remove", tint: palette.red) }
        }
        .padding(.horizontal, 14).padding(.vertical, 10)
        .background(hover ? palette.link.opacity(0.10) : .clear)
        .overlay(alignment: .bottom) {
            if divider {
                Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, look.textMarks ? 0 : 56)
            }
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
        .foregroundStyle(look.palette.link).padding(.horizontal, 14).frame(height: 48)
    }
}

internal struct RulesBody: View {
    @Environment(\.look) private var look
    internal let name: String
    internal let rules: [RuleFixture]
    internal var agent = ""
    internal var fresh: String?
    internal var hover: String?
    internal var swiped: String?

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 22) {
            VStack(alignment: .leading, spacing: 8) {
                Group2(header: "\(name)'s rules") {
                    ForEach(rules) { rule in
                        line(rule).background(rule.id == fresh ? palette.green.opacity(0.10) : .clear)
                    }
                    AddRuleRow()
                }
                Text("When several match, the strictest wins: deny, then ask, then allow.")
                    .ui(12.5, .regular, look).foregroundStyle(palette.mute).padding(.horizontal, 16)
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

    @ViewBuilder private func line(_ rule: RuleFixture) -> some View {
        let row = RuleLine(rule: rule, agent: agent, hover: rule.id == hover)
        if rule.id == swiped {
            ZStack(alignment: .trailing) {
                look.palette.red
                Text(look.label(.button, "Remove")).ui(15, .semibold, look).foregroundStyle(.white)
                    .frame(width: 96)
                row.background(look.palette.card).offset(x: -96)
            }
            .clipped()
        } else {
            row
        }
    }
}

internal struct BackBar: View {
    @Environment(\.look) private var look
    internal let back: String
    internal let title: String

    internal var body: some View {
        HStack {
            Text(look.label(.back, back)).ui(16, .regular, look).foregroundStyle(look.palette.link)
            Spacer()
        }
        .overlay { Text(look.label(.section, title)).ui(17, .semibold, look) }
        .padding(.horizontal, 16).frame(height: 44)
    }
}

internal struct RulesPage: View {
    @Environment(\.look) private var look
    internal var rules = Story.chief
    internal var fresh: String?
    internal var swiped: String?

    internal var body: some View {
        VStack(spacing: 0) {
            BackBar(back: "Chief", title: "Rules")
            Scroller {
                RulesBody(name: "Chief", rules: rules, fresh: fresh, swiped: swiped)
                    .padding(.horizontal, 16).padding(.vertical, 12)
            }
        }
        .background(Backdrop()).foregroundStyle(look.palette.foreground)
    }
}
