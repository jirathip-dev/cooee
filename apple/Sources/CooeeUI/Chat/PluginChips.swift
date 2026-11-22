import SwiftUI

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
