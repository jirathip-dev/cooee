import CooeeCore
import SwiftUI

internal struct RoundRow: View {
    @Environment(\.look) private var look
    internal let round: ListedChat
    internal let current: Bool
    internal let open: (String) -> Void

    internal var body: some View {
        let (palette, row) = (look.palette, round.row)
        HStack(alignment: .top, spacing: 14) {
            Circle().fill(palette.state(row.state)).frame(width: current ? 12 : 9, height: current ? 12 : 9)
                .frame(width: 12, height: 24)
            VStack(alignment: .leading, spacing: 8) {
                Button {
                    open(row.id)
                } label: {
                    HStack(spacing: 8) {
                        Text(round.workflow?.stage ?? row.title).ui(15, .semibold, look)
                        Text(row.status ?? row.state.word).ui(14, .regular, look)
                            .foregroundStyle(palette.state(row.state))
                        Spacer()
                        Text("\(row.ago) ›").ui(14, .regular, look).foregroundStyle(palette.mute)
                    }
                    .frame(minHeight: 24).contentShape(.rect)
                }
                .buttonStyle(.plain)
                if current { tools }
            }
            .padding(.bottom, 14)
        }
        .background(alignment: .leading) { Rectangle().fill(palette.line).frame(width: 2).padding(.leading, 5) }
    }

    private var tools: some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 10) {
            if !round.row.last.isEmpty {
                Text(round.row.last).font(.system(size: 13, design: .monospaced)).foregroundStyle(palette.mute)
                    .lineLimit(3)
            }
            HStack(spacing: 8) {
                Button("open chat ›") { open(round.row.id) }.buttonStyle(.plain).modifier(OutlineChip())
                if let link = round.workflow?.link {
                    Link("\(link.title) ›", destination: link.url).modifier(OutlineChip())
                }
            }
        }
        .padding(12).frame(maxWidth: .infinity, alignment: .leading)
        .background(palette.card, in: .rect(cornerRadius: 14))
        .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(palette.line, lineWidth: 1))
    }
}

internal struct LaneStatusRow: View {
    @Environment(\.look) private var look
    internal let status: String
    internal let close: (() -> Void)?

    internal var body: some View {
        HStack(spacing: 8) {
            Text(status).ui(14, .regular, look).foregroundStyle(look.palette.mute).lineLimit(1)
            Spacer(minLength: 4)
            if let close {
                Button("close", action: close).buttonStyle(.plain).modifier(OutlineChip())
                    .accessibilityLabel("Close session")
            }
        }
    }
}
