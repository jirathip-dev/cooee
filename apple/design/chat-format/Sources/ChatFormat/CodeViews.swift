import SwiftUI

internal struct CodeView: View {
    @Environment(\.look) private var look
    internal let lang: String
    internal let code: String

    internal var body: some View {
        let palette = look.palette
        let longest = code.split(separator: "\n").map(\.count).max() ?? 0
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text(look.mono ? "── \(lang) " : lang).font(look.small(.medium)).foregroundStyle(palette.mute)
                Spacer()
                CopyLabel()
            }
            .padding(.horizontal, 12).frame(height: 30).background(palette.edge.opacity(0.3))
            Text(Syntax.colour(code, lang: lang, palette)).font(look.code()).lineSpacing(2).fixedSize()
                .padding(12).frame(width: look.column, alignment: .leading).clipped()
                .fadesRight(look, when: !look.fits(longest, width: look.column - 24))
        }
        .boxed(look)
    }
}

internal struct CopyLabel: View {
    @Environment(\.look) private var look

    internal var body: some View {
        if look.mono {
            Text("[copy]").font(look.small(.medium)).foregroundStyle(look.palette.link)
        } else {
            Label("Copy", systemImage: "doc.on.doc").font(look.small(.medium)).foregroundStyle(look.palette.link)
        }
    }
}

internal struct FoldRow: View {
    @Environment(\.look) private var look
    internal let count: Int
    internal let earlier: Bool

    internal var body: some View {
        let words = earlier ? "\(count) earlier lines" : "\(count) more lines"
        HStack(spacing: 6) {
            if look.mono {
                Text(earlier ? "[^ \(words)]" : "[v \(words)]")
            } else {
                Image(systemName: earlier ? "chevron.up" : "chevron.down").font(.system(size: 11, weight: .bold))
                Text(words)
            }
        }
        .font(look.small(.medium)).foregroundStyle(look.palette.link)
        .frame(maxWidth: .infinity).frame(height: 30).background(look.palette.edge.opacity(0.2))
    }
}

internal struct DiffView: View {
    @Environment(\.look) private var look
    internal let diff: Diff

    internal var body: some View {
        let folded = diff.lines.count > Fold.limit
        VStack(alignment: .leading, spacing: 0) {
            header
            VStack(alignment: .leading, spacing: 0) {
                ForEach(Array(diff.lines.prefix(folded ? Fold.shown : diff.lines.count).enumerated()), id: \.offset) {
                    line($0.element)
                }
            }
            .padding(.vertical, 4).frame(width: look.column, alignment: .leading).clipped()
            if folded { FoldRow(count: diff.lines.count - Fold.shown, earlier: false) }
        }
        .boxed(look)
    }

    private var header: some View {
        let palette = look.palette
        let name = (diff.file as NSString).lastPathComponent
        let folder = (diff.file as NSString).deletingLastPathComponent
        return HStack(spacing: 6) {
            if look.mono {
                Text(diff.edit ? "~" : "±").foregroundStyle(palette.amber)
            } else {
                Image(systemName: diff.edit ? "pencil" : "doc.text").foregroundStyle(palette.mute)
            }
            if diff.edit { Text("Edited").foregroundStyle(palette.mute) }
            Text(name).fontWeight(.semibold)
            Text(folder).foregroundStyle(palette.mute).lineLimit(1).truncationMode(.head)
            Spacer(minLength: 6)
            Text("+\(diff.added)").foregroundStyle(palette.green)
            Text("−\(diff.removed)").foregroundStyle(palette.red)
        }
        .font(look.code()).padding(.horizontal, 12).frame(height: 34).background(palette.edge.opacity(0.3))
    }

    private func line(_ line: String) -> some View {
        let palette = look.palette
        let sign = line.first ?? " "
        let tint: Color = sign == "+" ? palette.green : sign == "-" ? palette.red : sign == "@" ? palette.mute : .clear
        let text = sign == "@" ? line : String(line.dropFirst())
        return HStack(spacing: 0) {
            Text(sign == "@" ? "" : sign == "-" ? "−" : String(sign)).foregroundStyle(tint).frame(width: 22)
            Text(text).foregroundStyle(sign == "@" ? palette.mute : palette.foreground).fixedSize()
        }
        .font(look.code()).frame(maxWidth: .infinity, minHeight: 19, alignment: .leading)
        .background(sign == "+" || sign == "-" ? tint.opacity(palette.dark ? 0.16 : 0.12) : .clear)
    }
}

internal struct RunView: View {
    @Environment(\.look) private var look
    internal let run: Run

    internal var body: some View {
        let palette = look.palette
        let folded = run.output.count > Fold.limit
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                Text("$").foregroundStyle(palette.green).fontWeight(.bold)
                Text(run.command).fontWeight(.semibold).lineLimit(1)
                Spacer(minLength: 6)
                Text(String(format: "%.1fs", run.seconds)).foregroundStyle(palette.mute)
                exit
            }
            .font(look.code()).padding(.horizontal, 12).frame(height: 34).background(palette.edge.opacity(0.3))
            if folded { FoldRow(count: run.output.count - Fold.shown, earlier: true) }
            VStack(alignment: .leading, spacing: 2) {
                ForEach(Array(run.output.suffix(folded ? Fold.shown : run.output.count).enumerated()), id: \.offset) {
                    Text(ANSI.text($0.element, look)).font(look.code()).fixedSize()
                }
            }
            .padding(12).frame(width: look.column, alignment: .leading).clipped()
        }
        .boxed(look)
    }

    private var exit: some View {
        let colour = run.exit == 0 ? look.palette.green : look.palette.red
        let label = "exit \(run.exit)"
        return Text(look.mono ? "[\(label)]" : label).font(look.small(.bold)).foregroundStyle(colour)
            .padding(.horizontal, look.mono ? 0 : 7).frame(height: 20)
            .background(look.mono ? .clear : colour.opacity(0.16), in: .capsule)
    }
}
