import CooeeCore
import SwiftUI

internal struct DoneButton: View {
    @Environment(\.look) private var look
    internal let done: JSONValue
    internal var values: [String: JSONValue] = [:]

    internal var body: some View {
        let off = done["off"]?.bool == true
        ActButton(source: done, values: values) {
            Text(done.string("label") ?? "").ui(16, .semibold, look)
                .foregroundStyle(off ? look.palette.mute : look.palette.link)
        }
        .disabled(off)
    }
}

internal struct PendingBar: View {
    @Environment(\.look) private var look
    internal let pending: JSONValue

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 12) {
            HStack(spacing: 0) {
                Text(pending.string("count") ?? "").fontWeight(.semibold).foregroundStyle(palette.lead)
                Text(" · \(pending.string("text") ?? "")")
            }
            .ui(13, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
            Spacer(minLength: 4)
            ActButton(source: .object(["tap": pending["discard"] ?? .null])) {
                Text("Discard").ui(13, .regular, look).foregroundStyle(palette.mute)
            }
            ActButton(source: .object(["tap": pending["review"] ?? .null])) {
                CardButton(title: "Review changes", primary: true, tint: palette.lead)
            }
        }
        .padding(.vertical, 8).padding(.leading, 16).padding(.trailing, 8)
        .surface(look, radius: 20).padding(12)
    }
}

internal struct DiffBlock: View {
    @Environment(\.look) private var look
    internal let block: JSONValue

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text(block.string("file") ?? "").fontWeight(.semibold)
                Spacer(minLength: 4)
                if let summary = block.string("summary") { Text(summary).foregroundStyle(palette.mute) }
            }
            .padding(.horizontal, 10).padding(.bottom, 4)
            ForEach(Array(block.list("hunks").enumerated()), id: \.offset) { _, hunk in
                Text(hunk.string("at") ?? "").foregroundStyle(palette.link).padding(.horizontal, 10).padding(
                    .vertical, 4)
                ForEach(Array(hunk.list("lines").compactMap(\.string).enumerated()), id: \.offset) { _, line in
                    DiffLine(line: line)
                }
            }
        }
        .font(.system(size: 12, design: .monospaced)).padding(.vertical, 8)
        .frame(maxWidth: .infinity, alignment: .leading).surface(look, radius: 12)
    }
}

internal struct DiffLine: View {
    @Environment(\.look) private var look
    internal let line: String

    internal var body: some View {
        let palette = look.palette
        let tint: Color? =
            switch line.first ?? " " {
            case "+": palette.green
            case "-": palette.red
            default: nil
            }
        Text(line).foregroundStyle(tint == nil ? palette.mute : palette.foreground)
            .padding(.horizontal, 10).frame(maxWidth: .infinity, alignment: .leading)
            .background(tint?.opacity(0.18) ?? .clear)
    }
}

internal struct EditorBlock: View {
    @Environment(\.look) private var look
    @Environment(\.act) private var act
    @State private var draft: String?
    internal let block: JSONValue

    private var text: String { block.string("text") ?? "" }
    private var first: Int { Int(block["first"]?.number ?? 1) }

    private var errors: [Int: String] {
        let pairs = block.list("errors").compactMap { error in
            error["line"]?.number.map { (Int($0), error.string("message") ?? "") }
        }
        return Dictionary(pairs) { kept, _ in kept }
    }

    internal var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if draft == nil {
                ForEach(Array(text.components(separatedBy: "\n").enumerated()), id: \.offset) { index, line in
                    EditorLine(number: first + index, text: line, error: errors[first + index])
                }
            } else {
                TextEditor(
                    text: Binding {
                        draft ?? ""
                    } set: {
                        draft = $0
                    }
                ).frame(minHeight: 320)
                    .scrollContentBackground(.hidden)
                ForEach(errors.sorted { $0.key < $1.key }, id: \.key) { error in
                    ErrorNote(error: "line \(error.key) · \(error.value)")
                }
            }
        }
        .font(.system(size: 12, design: .monospaced)).padding(.vertical, 8)
        .frame(maxWidth: .infinity, alignment: .leading).surface(look, radius: 12)
        .onTapGesture { if draft == nil { draft = text } }
        .task(id: draft) { await check() }
    }

    private func check() async {
        guard let draft, draft != text, let action = block.string("action") else { return }
        try? await Task.sleep(for: .milliseconds(400))
        guard !Task.isCancelled else { return }
        act(BlockAction(action: action, values: [block.string("name") ?? "": .string(draft)]))
    }
}

internal struct EditorLine: View {
    @Environment(\.look) private var look
    internal let number: Int
    internal let text: String
    internal let error: String?

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 2) {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                Text("\(number)").foregroundStyle(palette.mute).frame(width: 30, alignment: .trailing)
                Text(text.isEmpty ? " " : text).foregroundStyle(palette.foreground)
                Spacer(minLength: 0)
            }
            .background(error == nil ? Color.clear : palette.red.opacity(0.14))
            if let error { ErrorNote(error: error).padding(.leading, 30) }
        }
    }
}

internal struct ErrorNote: View {
    @Environment(\.look) private var look
    internal let error: String

    internal var body: some View {
        Text(error).ui(11.5, .semibold, look).foregroundStyle(look.palette.red)
            .padding(.vertical, 4).padding(.horizontal, 8)
            .background(look.palette.red.opacity(0.18), in: .rect(cornerRadius: 6)).padding(.horizontal, 10)
    }
}

internal struct Reordering: ViewModifier {
    @Environment(\.act) private var act
    internal let list: JSONValue
    internal let row: JSONValue

    @ViewBuilder
    internal func body(content: Content) -> some View {
        if let reorder = list["reorder"], let id = row.id, row["handle"]?.bool == true {
            content.draggable(id).dropDestination(for: String.self) { dropped, _ in
                guard let moved = dropped.first, let action = reorder.string("action") else { return }
                let order = list.rowOrder(moving: moved, onto: id).map(JSONValue.string)
                act(BlockAction(action: action, values: [reorder.string("name") ?? "": .array(order)]))
            }
        } else {
            content
        }
    }
}
