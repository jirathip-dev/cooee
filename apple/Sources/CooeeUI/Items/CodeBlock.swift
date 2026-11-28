import CooeeCore
import SwiftUI

internal struct FenceView: View {
    internal let lang: String
    internal let code: String

    internal var body: some View {
        switch Fence(lang: lang, code) {
        case .source: CodeBlock(lang: lang, code: code)
        case .diff(let diff): DiffCard(diff: diff)
        case .output(let lines): OutputCard(lang: lang, lines: lines)
        }
    }
}

internal struct CodeBlock: View {
    @Environment(\.look) private var look
    internal let lang: String
    internal let code: String

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text(look.mono ? "── \(lang) " : lang).ui(12, .medium, look).foregroundStyle(palette.mute)
                Spacer()
                CopyButton(text: code)
            }
            .padding(.horizontal, 12).frame(height: 30).background(palette.edge.opacity(0.3))
            Sideways {
                Text(Syntax.colour(code, lang: lang, palette)).font(.system(size: 13, design: .monospaced))
                    .lineSpacing(2).padding(12)
            }
        }
        .boxed(look)
    }
}

internal struct CopyButton: View {
    @Environment(\.look) private var look
    internal let text: String

    internal var body: some View {
        Button {
            copy(text)
        } label: {
            Mark(text: "[copy]", size: 12, weight: .medium) {
                Label("Copy", systemImage: "doc.on.doc").ui(12, .medium, look)
            }
            .foregroundStyle(look.palette.link)
        }
        .buttonStyle(.plain)
    }
}

internal struct FoldRow: View {
    @Environment(\.look) private var look
    internal let count: Int
    internal let earlier: Bool
    internal let open: () -> Void

    internal var body: some View {
        let words = earlier ? "\(count) earlier lines" : "\(count) more lines"
        Button(action: open) {
            HStack(spacing: 6) {
                Mark(text: earlier ? "[^ \(words)]" : "[v \(words)]", size: 12, weight: .medium) {
                    Image(systemName: earlier ? "chevron.up" : "chevron.down").font(.system(size: 11, weight: .bold))
                    Text(words)
                }
            }
            .ui(12, .medium, look).foregroundStyle(look.palette.link)
            .frame(maxWidth: .infinity).frame(height: 30).background(look.palette.edge.opacity(0.2))
        }
        .buttonStyle(.plain)
    }
}

internal struct DiffCard: View {
    @Environment(\.look) private var look
    @State private var open = false
    internal let diff: Diff

    internal var body: some View {
        let folded = Folded.head(diff.lines)
        VStack(alignment: .leading, spacing: 0) {
            header
            Sideways {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(Array((open ? diff.lines : folded.lines).enumerated()), id: \.offset) { line($0.element) }
                }
                .padding(.vertical, 4)
            }
            if folded.hidden > 0, !open { FoldRow(count: folded.hidden, earlier: false) { open = true } }
        }
        .boxed(look)
    }

    private var header: some View {
        let palette = look.palette
        let path = diff.file as NSString
        return HStack(spacing: 6) {
            Mark(text: "±", size: 13) { Image(systemName: "doc.text") }.foregroundStyle(
                look.mono ? palette.amber : palette.mute)
            Text(diff.file.isEmpty ? "diff" : path.lastPathComponent).fontWeight(.semibold)
            Text(path.deletingLastPathComponent).foregroundStyle(palette.mute).lineLimit(1).truncationMode(.head)
            Spacer(minLength: 6)
            Text("+\(diff.added)").foregroundStyle(palette.green)
            Text("−\(diff.removed)").foregroundStyle(palette.red)
        }
        .font(.system(size: 13, design: .monospaced)).padding(.horizontal, 12).frame(height: 34)
        .background(palette.edge.opacity(0.3))
    }

    private func line(_ line: String) -> some View {
        let palette = look.palette
        let sign = line.first ?? " "
        let tint: Color = sign == "+" ? palette.green : sign == "-" ? palette.red : sign == "@" ? palette.mute : .clear
        return HStack(spacing: 0) {
            Text(sign == "@" ? "" : sign == "-" ? "−" : String(sign)).foregroundStyle(tint).frame(width: 22)
            Text(sign == "@" ? line : String(line.dropFirst())).foregroundStyle(
                sign == "@" ? palette.mute : palette.foreground
            )
            .padding(.trailing, 12)
        }
        .font(.system(size: 13, design: .monospaced)).frame(maxWidth: .infinity, minHeight: 19, alignment: .leading)
        .background(sign == "+" || sign == "-" ? tint.opacity(palette.dark ? 0.16 : 0.12) : .clear)
    }
}

internal struct OutputCard: View {
    @Environment(\.look) private var look
    @State private var open = false
    internal let lang: String
    internal let lines: [String]

    internal var body: some View {
        let palette = look.palette
        let folded = Folded.tail(lines)
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                Text("$").foregroundStyle(palette.green).fontWeight(.bold)
                Text(lang).foregroundStyle(palette.mute)
                Spacer(minLength: 6)
                CopyButton(text: lines.map { ANSI.runs($0).map(\.text).joined() }.joined(separator: "\n"))
            }
            .font(.system(size: 13, design: .monospaced)).padding(.horizontal, 12).frame(height: 34)
            .background(palette.edge.opacity(0.3))
            if folded.hidden > 0, !open { FoldRow(count: folded.hidden, earlier: true) { open = true } }
            Sideways {
                VStack(alignment: .leading, spacing: 2) {
                    ForEach(Array((open ? lines : folded.lines).enumerated()), id: \.offset) {
                        Text(Syntax.ansi($0.element, palette))
                    }
                }
                .padding(12)
            }
        }
        .boxed(look)
    }
}
