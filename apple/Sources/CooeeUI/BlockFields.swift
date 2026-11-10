import CooeeCore
import SwiftUI

internal struct RunBlock: View {
    @Environment(\.look) private var look
    internal let block: JSONValue

    internal var body: some View {
        let palette = look.palette
        let command = block.string("command") ?? ""
        let approve: JSONValue = .object([
            "confirm": .object(["title": .string("Run it?"), "message": .string(command), "ok": .string("Run")])
        ])
        VStack(alignment: .leading, spacing: 10) {
            Text("$ \(command)").font(.system(size: 13, design: .monospaced)).foregroundStyle(palette.foreground)
                .padding(10).frame(maxWidth: .infinity, alignment: .leading)
                .background(palette.background.opacity(0.6), in: .rect(cornerRadius: 10))
            VStack(alignment: .leading, spacing: 3) {
                Text("sha \(block.string("sha") ?? "")").font(.system(size: 12, design: .monospaced))
                if let dir = block.string("dir") { Text("in \(dir)") }
                if let touches = block.string("touches") { Text("touches \(touches)") }
            }
            .ui(12, .regular, look).foregroundStyle(palette.mute)
            HStack(spacing: 8) {
                answer("approve", source: approve) { CardButton(title: "Approve", primary: true, tint: palette.green) }
                answer("deny", source: .null) { CardButton(title: "Deny", tint: palette.red) }
            }
            if let until = block["until"]?.date {
                RelativeText(template: "lapses in {rel}", until: until).ui(12, .regular, look).foregroundStyle(
                    palette.amber)
            }
        }
    }

    private func answer(_ choice: String, source: JSONValue, label: () -> some View) -> some View {
        var values: [String: JSONValue] = ["choice": .string(choice)]
        if choice == "approve" { values["sha"] = block["sha"] }
        return ActButton(block: block.id ?? "", action: block.string("action"), source: source, values: values) {
            label()
        }
    }
}

internal struct SecretBlock: View {
    @Environment(\.look) private var look
    @Environment(\.card) private var context
    @State private var value = ""
    @State private var editing = false
    internal let block: JSONValue

    internal var body: some View {
        let palette = look.palette
        let name = block.string("name") ?? ""
        let poster = context.poster.map { "the \($0) plugin" } ?? "its plugin"
        VStack(alignment: .leading, spacing: 10) {
            Button {
                editing = true
            } label: {
                HStack {
                    Text(masked).ui(15, .regular, look).foregroundStyle(palette.mute)
                    Spacer()
                    Image(systemName: "faceid").foregroundStyle(palette.mute)
                }
                .padding(.horizontal, 12).frame(height: 38)
                .background(palette.background.opacity(0.5), in: .rect(cornerRadius: 10))
            }
            .buttonStyle(.plain).accessibilityLabel(block.string("label") ?? name)
            Label("Sent to \(poster) only, never to the agent", systemImage: "lock.fill")
                .ui(12, .regular, look).foregroundStyle(palette.mute)
            if value.isEmpty {
                Button {
                    editing = true
                } label: {
                    send
                }.buttonStyle(.plain)
            } else {
                ActButton(block: block.id ?? "", source: block, values: [name: .string(value)]) { send }
            }
        }
        .alert(block.string("label") ?? name, isPresented: $editing) {
            SecureField(block.string("placeholder") ?? name, text: $value)
            Button("OK") {}
        }
    }

    private var send: some View {
        CardButton(title: "Send to plugin", primary: true, tint: context.tint ?? look.palette.amber)
    }

    private var masked: String {
        if !value.isEmpty { return String(repeating: "•", count: 12) }
        return block["set"]?.bool == true ? "set ✓ · paste to replace" : "••••••••••••"
    }
}

internal struct FieldsBlock: View {
    @Environment(\.look) private var look
    @Environment(\.card) private var context
    @Environment(\.form) private var shared
    @State private var own: [String: JSONValue] = [:]
    internal let block: JSONValue

    internal var body: some View {
        let edits = block["submit"] == nil ? shared ?? $own : $own
        VStack(alignment: .leading, spacing: 10) {
            ForEach(Array(block.list("fields").enumerated()), id: \.offset) { _, field in
                FieldRow(field: field, edits: edits)
            }
            if let submit = block["submit"] {
                ActButton(block: block.id ?? "", source: submit, values: block.fieldValues.merging(own) { $1 }) {
                    CardButton(title: submit.string("label") ?? "", primary: true, tint: context.tint)
                }
            }
        }
    }
}

internal struct FieldRow: View {
    @Environment(\.look) private var look
    @State private var editing = false
    @State private var draft = ""
    internal let field: JSONValue
    @Binding internal var edits: [String: JSONValue]

    private var name: String { field.string("name") ?? "" }
    private var value: JSONValue? { edits[name] ?? field["value"] }

    private var shown: String {
        switch field.type {
        case "toggle": value?.bool == true ? "on" : "off"
        case "secret": edits[name] == nil ? (field["set"]?.bool == true ? "set ✓" : "") : "••••••••"
        default: value?.text(at: .now) ?? field.string("placeholder") ?? ""
        }
    }

    internal var body: some View {
        let palette = look.palette
        Button(action: tap) {
            HStack {
                Text(field.string("label") ?? name).ui(14, .regular, look).foregroundStyle(palette.mute)
                Spacer()
                Text(shown).ui(14, .regular, look)
            }
            .padding(.horizontal, 12).frame(height: 38)
            .background(palette.background.opacity(0.5), in: .rect(cornerRadius: 10))
        }
        .buttonStyle(.plain)
        .alert(field.string("label") ?? name, isPresented: typing) { editor }
        .confirmationDialog(field.string("label") ?? name, isPresented: picking) {
            ForEach(field.list("options").compactMap(\.string), id: \.self) { option in
                Button(option) { edits[name] = .string(option) }
            }
        }
    }

    private var typing: Binding<Bool> {
        Binding(get: { editing && field.type != "picker" }, set: { editing = $0 })
    }

    private var picking: Binding<Bool> {
        Binding(get: { editing && field.type == "picker" }, set: { editing = $0 })
    }

    @ViewBuilder private var editor: some View {
        if field.type == "secret" {
            SecureField(name, text: $draft)
        } else {
            TextField(field.string("placeholder") ?? name, text: $draft)
        }
        Button("OK") { edits[name] = .string(draft) }
        Button("Cancel", role: .cancel) {}
    }

    private func tap() {
        if field.type == "toggle" {
            edits[name] = .bool(value?.bool != true)
        } else {
            draft = field.type == "secret" ? "" : value?.string ?? ""
            editing = true
        }
    }
}
