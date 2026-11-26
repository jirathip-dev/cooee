import SwiftUI

internal struct RowView: View {
    @Environment(\.look) private var look
    internal let row: Row

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 8) {
            kind
            subject
            Spacer(minLength: 6)
            state
        }
        .padding(.horizontal, 12).frame(height: 34)
        .background(fill, in: .rect(cornerRadius: radius))
        .overlay {
            if look.mono {
                Rectangle().strokeBorder(row.status == .failed ? palette.red.opacity(0.6) : palette.line, lineWidth: 1)
            } else if row.status == .failed || look.style == .scene {
                RoundedRectangle(cornerRadius: radius).strokeBorder(
                    row.status == .failed ? palette.red.opacity(0.55) : palette.edge,
                    lineWidth: look.style == .scene ? 1.5 : 1)
            }
        }
    }

    private var radius: CGFloat { look.mono ? 0 : look.style == .scene ? 8 : 10 }

    private var fill: Color {
        let palette = look.palette
        if look.mono { return row.status == .failed ? palette.red.opacity(0.08) : .clear }
        return row.status == .failed ? palette.red.opacity(palette.dark ? 0.12 : 0.08) : palette.card.opacity(0.7)
    }

    @ViewBuilder private var kind: some View {
        let palette = look.palette
        if look.mono {
            Text(row.kind.ascii.padding(toLength: 6, withPad: " ", startingAt: 0))
                .font(.system(size: 13, weight: .bold, design: .monospaced)).foregroundStyle(palette.lead).fixedSize()
        } else {
            Image(systemName: row.kind.symbol).font(.system(size: 12, weight: .semibold))
                .foregroundStyle(palette.lead).frame(width: 18)
            if !row.kind.word.isEmpty {
                Text(row.kind.word).font(look.font(13, .semibold)).foregroundStyle(palette.foreground).fixedSize()
            }
        }
    }

    private var subject: some View {
        let mono = look.mono || row.kind.monoSubject
        let text = row.kind == .run && look.mono ? "$ \(row.subject)" : row.subject
        return Text(text)
            .font(.system(size: mono ? 12.5 : 13, design: mono ? .monospaced : look.design))
            .foregroundStyle(look.palette.mute).lineLimit(1).truncationMode(row.kind.path ? .head : .tail)
    }

    @ViewBuilder private var state: some View {
        let palette = look.palette
        switch row.status {
        case .running:
            mark(ascii: "[…]", symbol: "circle.dotted", word: "running", tint: palette.cyan)
        case .done:
            mark(ascii: "[ok]", symbol: "checkmark.circle.fill", word: nil, tint: palette.green)
        case .failed:
            mark(ascii: "[failed]", symbol: "xmark.octagon.fill", word: "failed", tint: palette.red)
        }
    }

    @ViewBuilder private func mark(ascii: String, symbol: String, word: String?, tint: Color) -> some View {
        if look.mono {
            Text(ascii).font(.system(size: 12.5, weight: .bold, design: .monospaced)).foregroundStyle(tint)
                .fixedSize()
        } else {
            HStack(spacing: 5) {
                if let word { Text(word).font(look.font(12, .medium)) }
                Image(systemName: symbol).font(.system(size: 13))
            }
            .foregroundStyle(tint).fixedSize()
        }
    }
}

internal struct FoldView: View {
    @Environment(\.look) private var look
    internal let rows: [Row]
    internal let open: Bool

    internal var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            if let last = rows.last {
                HStack(spacing: 6) {
                    RowView(row: last)
                    count
                }
            }
            if open {
                ForEach(Array(rows.dropLast().enumerated()), id: \.offset) { _, earlier in
                    RowView(row: earlier).padding(.leading, look.mono ? 4 : 12)
                }
            }
        }
    }

    private var count: some View {
        let palette = look.palette
        return HStack(spacing: 4) {
            Text(look.mono ? "\(rows.count)" : "\(rows.count) tools").font(look.font(12))
            Text(look.mono ? (open ? "[v]" : "[>]") : "")
            if !look.mono {
                Image(systemName: open ? "chevron.down" : "chevron.right").font(.system(size: 10, weight: .bold))
            }
        }
        .foregroundStyle(palette.mute).fixedSize()
    }
}

internal struct Specimen: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 6) {
            Text(look.mono ? "── every kind: running, done, failed" : "Every kind: running, done, failed")
                .font(look.font(12, .semibold)).foregroundStyle(palette.mute).padding(.bottom, 2)
            ForEach(Kind.allCases, id: \.self) { kind in
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(Array(Fixture.specimen.filter { $0.kind == kind }.enumerated()), id: \.offset) {
                        RowView(row: $0.element)
                    }
                }
                .padding(.bottom, 6)
            }
        }
    }
}
