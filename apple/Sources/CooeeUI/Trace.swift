import CooeeCore
import SwiftUI

internal struct TraceRow: View {
    @Environment(\.look) private var look
    @State private var open = false
    internal let steps: [ToolRow]
    internal let seconds: Int

    internal var body: some View {
        Button {
            open = true
        } label: {
            HStack(spacing: 6) {
                Text("Worked \(seconds < 60 ? "\(seconds)s" : "\(seconds / 60)m") · \(steps.count) steps")
                Image(systemName: "chevron.right").font(.system(size: 10, weight: .bold))
            }
            .ui(13, .regular, look).foregroundStyle(look.palette.mute).contentShape(.rect)
        }
        .buttonStyle(.plain)
        .sheet(isPresented: $open) { TraceSheet(steps: steps) }
    }
}

internal struct TraceSheet: View {
    @Environment(\.look) private var look
    @Environment(\.dismiss) private var dismiss
    internal let steps: [ToolRow]

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("\(steps.count) steps").ui(17, .semibold, look)
                Spacer()
                Button {
                    dismiss()
                } label: {
                    RoundButton(symbol: "xmark", ascii: "x", size: 36)
                }
                .buttonStyle(.plain).accessibilityLabel("Close")
            }
            .padding(16)
            Scroller {
                Group2 {
                    ForEach(Array(steps.enumerated()), id: \.offset) { index, step in row(index, step) }
                }
                .padding(.horizontal, 16)
            }
        }
        .background(palette.background).foregroundStyle(palette.foreground)
    }

    private func row(_ index: Int, _ step: ToolRow) -> some View {
        let palette = look.palette
        let failed = step.status == "failed"
        let tint = step.running ? palette.cyan : failed ? palette.red : palette.green
        return HStack(spacing: 10) {
            Text("\(index + 1)").ui(13, .medium, look).foregroundStyle(palette.mute).frame(width: 20)
            if look.mono {
                Text(step.running ? "o>_" : failed ? "[x]" : "[ok]").ui(13, .bold, look).foregroundStyle(tint)
            } else {
                Image(
                    systemName: step.running
                        ? "gearshape.2.fill" : failed ? "xmark.circle.fill" : "checkmark.circle.fill"
                )
                .font(.system(size: 13)).foregroundStyle(tint)
            }
            Text(step.title).ui(14, .regular, look).lineLimit(2).truncationMode(.middle)
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 14).padding(.vertical, 10)
    }
}
