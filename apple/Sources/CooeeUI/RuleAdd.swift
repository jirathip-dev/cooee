import CooeeCore
import SwiftUI

extension ToolKind {
    internal var choice: String { word.isEmpty ? "Other" : word }

    internal var pattern: (label: String, hint: String) {
        switch self {
        case .read, .edit, .search: ("Files", "A path in the folder; ** for any file.")
        case .run: ("Command", "Its first words; end with * for anything after them.")
        case .fetch: ("Host", "A host, like docs.rs; * for any part of it.")
        case .tool: ("Tool", "A connector's server/tool, like github/delete_repo.")
        case .other: ("Title", "The request's title as the card shows it.")
        }
    }
}

internal struct AddRuleSheet: View {
    @Environment(\.look) private var look
    @Binding internal var draft: NewRule
    internal let folder: String?
    internal var error: String?
    internal var busy = false
    internal var save: () -> Void = {}
    internal var cancel: () -> Void = {}

    internal var body: some View {
        let palette = look.palette
        BottomSheet {
            RuleSheetBar(title: "New rule", cancel: cancel, busy: busy, run: save)
            RuleField(label: "Answer") {
                HStack(spacing: 8) {
                    ForEach(RuleEffect.allCases, id: \.self) { effect in
                        RuleChoice(title: effect.title, picked: effect == draft.effect, tint: effect.tint(palette)) {
                            draft.effect = effect
                        }
                    }
                }
            }
            RuleField(label: "When it wants to") { kinds }
            RuleField(label: draft.kind.pattern.label, hint: draft.kind.pattern.hint, error: error) { pattern }
            RuleField(label: "In") { place }
        }
    }

    private var kinds: some View {
        let all = ToolKind.allCases
        return VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 8) { ForEach(all.prefix(4), id: \.self) { kind($0) } }
            HStack(spacing: 8) { ForEach(all.dropFirst(4), id: \.self) { kind($0) } }
        }
    }

    private func kind(_ kind: ToolKind) -> some View {
        RuleChoice(title: kind.choice, symbol: kind.symbol, picked: kind == draft.kind) { draft.kind = kind }
    }

    private var pattern: some View {
        let palette = look.palette
        return HStack {
            if look.still {
                Text(draft.pattern)
            } else {
                TextField(draft.kind.pattern.label, text: $draft.pattern).autocorrectionDisabled()
                    .accessibilityLabel("Pattern")
            }
            Spacer()
        }
        .font(.system(size: 16, design: .monospaced))
        .padding(.horizontal, 12).frame(height: 44)
        .chip(look, fill: palette.card, edge: palette.edge, radius: look.radius(10), paper: palette.card)
    }

    private var place: some View {
        HStack(spacing: 8) {
            if let folder {
                RuleChoice(title: folder, symbol: "folder", picked: draft.folder == folder) { draft.folder = folder }
            }
            RuleChoice(title: "Any folder", picked: draft.folder == nil) { draft.folder = nil }
        }
    }
}
