import CooeeCore
@testable import CooeeUI
import SwiftUI

internal struct BottomSheet<Content: View>: View {
    @Environment(\.look) private var look
    @ViewBuilder internal var content: Content

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 16) {
            if look.textMarks {
                Text("── ── ──").ui(11, .bold, look).foregroundStyle(palette.edge).frame(maxWidth: .infinity)
            } else {
                Capsule().fill(palette.mute.opacity(0.5)).frame(width: 38, height: 5).frame(maxWidth: .infinity)
            }
            content
        }
        .padding(.horizontal, 16).padding(.top, 8).padding(.bottom, 34)
        .frame(maxWidth: .infinity, alignment: .leading)
        .chip(look, fill: palette.background, edge: palette.edge, radius: look.radius(30), paper: palette.background)
        .shadow(color: .black.opacity(0.25), radius: 18, y: -4)
        .foregroundStyle(palette.foreground)
    }
}

internal struct SheetBar: View {
    @Environment(\.look) private var look
    internal let title: String
    internal var done = "Add"

    internal var body: some View {
        HStack {
            Text(look.label(.button, "Cancel")).ui(16, .regular, look).foregroundStyle(look.palette.link)
            Spacer()
            Text(look.label(.button, done)).ui(16, .semibold, look).foregroundStyle(look.palette.link)
        }
        .overlay { Text(look.label(.section, title)).ui(17, .semibold, look) }
    }
}

internal struct Choice: View {
    @Environment(\.look) private var look
    internal let title: String
    internal var symbol: String?
    internal var ascii = ""
    internal let picked: Bool
    internal var tint: Color?

    internal var body: some View {
        let palette = look.palette
        let tint = tint ?? palette.link
        HStack(spacing: 6) {
            if let symbol, !look.textMarks { Image(systemName: symbol).font(.system(size: 12, weight: .semibold)) }
            Text(picked ? look.label(.picked, title) : title).ui(14, picked ? .semibold : .regular, look)
        }
        .foregroundStyle(picked ? (look.outline ? tint : (palette.dark ? Color.black : .white)) : palette.foreground)
        .lineLimit(1).fixedSize().padding(.horizontal, 12).frame(height: 34)
        .background(picked && !look.outline ? tint : palette.edge.opacity(look.outline ? 0 : 0.35), in: look.pill)
        .overlay { if look.outline { Rectangle().strokeBorder(picked ? tint : palette.edge, lineWidth: 1) } }
    }
}

internal struct Field: View {
    @Environment(\.look) private var look
    internal let label: String
    internal var hint = ""
    @ViewBuilder internal var content: AnyView

    internal var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(look.label(.group, label)).ui(12, .medium, look).foregroundStyle(look.palette.mute)
            content
            if !hint.isEmpty {
                Text(hint).ui(12.5, .regular, look).foregroundStyle(look.palette.mute)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }
}

internal struct AddRuleSheet: View {
    @Environment(\.look) private var look
    internal var folder = "~/work"

    private static let kinds = ToolKind.allCases

    internal var body: some View {
        let palette = look.palette
        BottomSheet {
            SheetBar(title: "New rule")
            Field(label: "Answer") {
                AnyView(
                    HStack(spacing: 8) {
                        ForEach(Effect.allCases, id: \.self) {
                            Choice(title: $0.title, picked: $0 == .allow, tint: $0.tint(palette))
                        }
                    })
            }
            Field(label: "When it wants to") {
                AnyView(
                    FlowRows(items: Self.kinds.map { kind in
                        AnyView(Choice(title: kind.word.isEmpty ? "Other" : kind.word, symbol: kind.symbol, picked: kind == .run))
                    }))
            }
            Field(label: "Command", hint: "Its first words; end with * for anything after them.") {
                AnyView(
                    HStack {
                        Text("npm run lint *").font(.system(size: 16, design: .monospaced))
                        Spacer()
                    }
                    .padding(.horizontal, 12).frame(height: 44)
                    .chip(look, fill: palette.card, edge: palette.edge, radius: look.radius(10), paper: palette.card))
            }
            Field(label: "In", hint: "A rule for one folder applies there and in its sub-folders.") {
                AnyView(
                    HStack(spacing: 8) {
                        Choice(title: folder, symbol: "folder", picked: true)
                        Choice(title: "Any folder", picked: false)
                    })
            }
        }
    }
}

internal struct FlowRows: View {
    internal let items: [AnyView]

    internal var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 8) { ForEach(0..<4, id: \.self) { items[$0] } }
            HStack(spacing: 8) { ForEach(4..<items.count, id: \.self) { items[$0] } }
        }
    }
}

internal struct RemoveSheet: View {
    @Environment(\.look) private var look
    internal let rule: RuleFixture

    internal var body: some View {
        let palette = look.palette
        BottomSheet {
            VStack(alignment: .leading, spacing: 6) {
                Text(look.label(.section, "Remove this rule?")).ui(18, .semibold, look)
                Text(rule.words).ui(15.5, .regular, look)
                Text("The next request it matches asks you on a card again.").ui(14, .regular, look)
                    .foregroundStyle(palette.mute)
            }
            CardButton(title: "Remove rule", primary: true, tint: palette.red)
            CardButton(title: "Cancel")
        }
    }
}
