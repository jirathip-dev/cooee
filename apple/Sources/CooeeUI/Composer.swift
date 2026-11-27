import CooeeCore
import SwiftUI

internal struct Composer: View {
    @Environment(\.look) private var look
    @Environment(\.members) private var members
    private let menu = State(initialValue: false)
    private let sending = State(initialValue: false)
    @FocusState private var typing: Bool
    internal let hint: String
    internal var queued: [String] = []
    internal var draft = ""
    internal var attachments: [String] = []
    internal var actions = ChatActions()
    internal var rest: CGFloat = 0

    private var lifted: Bool {
        #if os(iOS)
            typing
        #else
            false
        #endif
    }

    private var chips: [String] { actions.send == nil ? attachments : actions.files }
    private var shown: String { actions.denying ? "Deny and say…" : queued.isEmpty ? hint : "Queue a message" }
    private var overlaid: Bool { !members.isEmpty || !(actions.voice.dictation?.heard.unsure.isEmpty ?? true) }

    internal var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(Array(queued.enumerated()), id: \.offset) { index, text in queuedRow(index, text) }
            if !chips.isEmpty { chipRow }
            if let unsure = actions.voice.dictation?.heard.unsure.first, !actions.voice.listening {
                Alternatives(unsure: unsure, pick: actions.voice.pick)
            }
            HStack(alignment: .bottom, spacing: look.textMarks ? 4 : 8) {
                menuButton
                field
                if talking { MicButton(voice: actions.voice) } else { sendButton }
            }
        }
        .padding(.bottom, lifted ? min(rest, 8) : rest)
    }

    private var talking: Bool {
        members.isEmpty
            && (actions.voice.listening || ((actions.draft?.wrappedValue ?? draft).isEmpty && chips.isEmpty))
    }

    private var field: some View {
        let palette = look.palette
        return HStack(alignment: .firstTextBaseline, spacing: 0) {
            if look.textMarks { Text("> ").ui(16, .bold, look).foregroundStyle(palette.lead) }
            if let dictation = actions.voice.dictation, dictation.listening {
                DictationWords(dictation: dictation)
            } else if let text = actions.draft {
                TextField(members.isEmpty ? shown : "", text: Self.keyed(text, send: send), axis: .vertical)
                    .ui(16, .regular, look).lineLimit(1...5).textFieldStyle(.plain).onSubmit(send).focused($typing)
                    .onKeyPress(.return, phases: .down) { press in
                        Self.pressed(shift: press.modifiers.contains(.shift), text, send: send)
                        return .handled
                    }
                    .onChange(of: typing) { _, now in if !now && sending.wrappedValue { post() } }
                    .onChange(of: actions.denying) { _, now in if now { typing = true } }
                    .foregroundStyle(overlaid ? .clear : palette.foreground).accessibilityLabel(shown)
                    .overlay(alignment: .topLeading) {
                        if overlaid { tokens(text.wrappedValue).allowsHitTesting(false) }
                    }
            } else {
                tokens(draft)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, look.textMarks ? 8 : 16).padding(.vertical, 10)
        .frame(minHeight: 42)
        .surface(look, radius: 21)
    }

    internal static func keyed(_ text: Binding<String>, send: @escaping () -> Void) -> Binding<String> {
        Binding {
            text.wrappedValue
        } set: { typed in
            let old = text.wrappedValue
            let lines = { (text: String) in text.count { $0 == "\n" } }
            let returned = lines(typed) == lines(old) + 1 && typed.filter { $0 != "\n" } == old.filter { $0 != "\n" }
            if returned { send() } else { text.wrappedValue = typed }
        }
    }

    internal static func pressed(shift: Bool, _ text: Binding<String>, send: () -> Void) {
        if shift { text.wrappedValue += "\n" } else { send() }
    }

    private func tokens(_ text: String) -> some View {
        let palette = look.palette
        let token = palette.link.opacity(0.2)
        let mentioned = BlockView.mentioned(
            AttributedString(text.isEmpty ? shown : text), members, palette.link, token: token)
        return Text(DictationWords.underlined(mentioned, actions.voice.dictation, palette.amber))
            .ui(16, .regular, look).foregroundStyle(text.isEmpty ? palette.mute : palette.foreground)
    }

    private var chipRow: some View {
        let palette = look.palette
        return HStack(spacing: 6) {
            ForEach(Array(chips.enumerated()), id: \.offset) { index, name in
                HStack(spacing: 6) {
                    Image(systemName: name.hasSuffix(".png") ? "photo" : "doc")
                    Text(name)
                    Button {
                        actions.detach(index)
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                    }
                    .buttonStyle(.plain)
                }
                .ui(12, .regular, look).foregroundStyle(palette.mute)
                .padding(.horizontal, 10).frame(height: 28).background(palette.card, in: .capsule)
            }
        }
    }

    private func queuedRow(_ index: Int, _ text: String) -> some View {
        let palette = look.palette
        return HStack(spacing: 8) {
            Image(systemName: "clock").font(.system(size: 12)).foregroundStyle(palette.mute)
            Text(text).ui(14, .regular, look).lineLimit(1)
            Spacer()
            Button {
                actions.steer(index)
            } label: {
                Text("Send now").ui(12, .semibold, look).foregroundStyle(palette.link)
            }
            .buttonStyle(.plain)
            Button {
                actions.unqueue(index)
            } label: {
                Image(systemName: "xmark").font(.system(size: 11, weight: .bold)).foregroundStyle(palette.mute)
            }
            .buttonStyle(.plain).accessibilityLabel("Take back")
        }
        .padding(.horizontal, 12).frame(height: 34)
        .background(palette.card.opacity(0.8), in: .rect(cornerRadius: 12))
        .contentShape(.rect).onTapGesture { actions.edit(index) }
        .accessibilityAction(named: "Edit") { actions.edit(index) }
    }

    private func send() {
        if typing {
            sending.wrappedValue = true
            typing = false
        } else {
            post()
        }
    }

    private func post() {
        sending.wrappedValue = false
        actions.send?()
    }
}

extension Composer {
    private var menuButton: some View {
        Button {
            menu.wrappedValue = true
        } label: {
            if look.textMarks {
                Text("[/]").ui(15, .bold, look).foregroundStyle(look.palette.foreground).frame(height: 42)
            } else {
                RoundButton(symbol: "line.3.horizontal", ascii: "/", size: 42)
            }
        }
        .buttonStyle(.plain).accessibilityLabel("Menu")
        .popover(isPresented: menu.projectedValue) {
            ComposerMenu(attach: actions.attach) { menu.wrappedValue = false }
        }
    }

    private var sendButton: some View {
        let palette = look.palette
        return Button(action: send) {
            if look.textMarks && !actions.sending {
                Text("[send]").ui(15, .bold, look).foregroundStyle(palette.link).frame(height: 42)
            } else {
                Group {
                    if actions.sending {
                        Spinner()
                    } else {
                        Image(systemName: "arrow.up").font(.system(size: 17, weight: .bold))
                    }
                }
                .foregroundStyle(palette.dark ? Color.black : .white)
                .frame(width: 42, height: 42)
                .background(palette.link, in: look.square ? AnyShape(Rectangle()) : AnyShape(Circle()))
            }
        }
        .buttonStyle(.plain).accessibilityLabel("Send")
    }
}
