import CooeeCore
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

internal struct RuleSheetBar: View {
    @Environment(\.look) private var look
    internal let title: String
    internal var done = "Add"
    internal var cancel: () -> Void = {}
    internal var busy = false
    internal let run: () -> Void

    internal var body: some View {
        HStack {
            Button(look.label(.button, "Cancel"), action: cancel).buttonStyle(.plain)
            Spacer()
            Button(look.label(.button, done), action: run).buttonStyle(.plain).fontWeight(.semibold).disabled(busy)
        }
        .ui(16, .regular, look).foregroundStyle(look.palette.link)
        .overlay { Text(look.label(.section, title)).ui(17, .semibold, look).foregroundStyle(look.palette.foreground) }
    }
}

internal struct RuleChoice: View {
    @Environment(\.look) private var look
    internal let title: String
    internal var symbol: String?
    internal let picked: Bool
    internal var tint: Color?
    internal let pick: () -> Void

    internal var body: some View {
        let palette = look.palette
        let tint = tint ?? palette.link
        Button(action: pick) {
            HStack(spacing: 6) {
                if let symbol, !look.textMarks { Image(systemName: symbol).font(.system(size: 12, weight: .semibold)) }
                Text(picked ? look.label(.picked, title) : title).ui(14, picked ? .semibold : .regular, look)
            }
            .foregroundStyle(picked ? (look.outline ? tint : palette.dark ? Color.black : .white) : palette.foreground)
            .lineLimit(1).fixedSize().padding(.horizontal, 12).frame(height: 34)
            .background(picked && !look.outline ? tint : palette.edge.opacity(look.outline ? 0 : 0.35), in: look.pill)
            .overlay { if look.outline { Rectangle().strokeBorder(picked ? tint : palette.edge, lineWidth: 1) } }
        }
        .buttonStyle(.plain)
    }
}

internal struct RuleField<Content: View>: View {
    @Environment(\.look) private var look
    internal let label: String
    internal var hint = ""
    internal var error: String?
    @ViewBuilder internal var content: Content

    internal var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(look.label(.group, label)).ui(12, .medium, look).foregroundStyle(look.palette.mute)
            content
            if let error {
                Text(error).ui(12.5, .regular, look).foregroundStyle(look.palette.red)
            } else if !hint.isEmpty {
                Text(hint).ui(12.5, .regular, look).foregroundStyle(look.palette.mute)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }
}

internal struct RemoveSheet: View {
    @Environment(\.look) private var look
    internal let rule: Rule
    internal var confirm: () -> Void = {}
    internal var cancel: () -> Void = {}

    internal var body: some View {
        let palette = look.palette
        BottomSheet {
            VStack(alignment: .leading, spacing: 6) {
                Text(look.label(.section, "Remove this rule?")).ui(18, .semibold, look)
                Text(rule.words).ui(15.5, .regular, look)
                Text("The next request it matches asks you on a card again.").ui(14, .regular, look)
                    .foregroundStyle(palette.mute)
            }
            Button(action: confirm) { CardButton(title: "Remove rule", primary: true, tint: palette.red) }
                .buttonStyle(.plain)
            Button(action: cancel) { CardButton(title: "Cancel") }.buttonStyle(.plain)
        }
    }
}
