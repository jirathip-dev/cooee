import SwiftUI

/// A chat: the persona header (avatar, name, agent chip; a tap on the name opens the persona sheet), the items, the
/// working pill with stop, and the composer (attachments, queued messages).
public struct ChatView: View {
    @Environment(\.look) var look
    let persona: Persona
    let items: [Item]
    var working = false
    var queued: [String] = []
    var draft = ""
    var back = true
    public init(personaID: String, items: [Item], working: Bool = false, queued: [String] = [], draft: String = "", back: Bool = true) {
        persona = Fixtures.persona(personaID); self.items = items; self.working = working; self.queued = queued
        self.draft = draft; self.back = back
    }
    public static func streaming(back: Bool = true) -> ChatView {
        ChatView(personaID: "coder", items: Fixtures.streaming, working: true, queued: Fixtures.queued, back: back)
    }
    public static func cards() -> ChatView { ChatView(personaID: "res", items: Fixtures.cards) }
    public static func conductor() -> ChatView { ChatView(personaID: "cond", items: Fixtures.conductor) }
    public static func moreCards() -> ChatView { ChatView(personaID: "res", items: Fixtures.moreCards) }

    public var body: some View {
        let p = look.palette
        ZStack {
            Backdrop()
            VStack(spacing: 0) {
                ChatHeader(persona: persona, back: back)
                Scroller(end: true) {
                    VStack(alignment: .leading, spacing: 14) {
                        ForEach(items) { ItemView(item: $0, persona: persona) }
                    }
                    .padding(.horizontal, 16).padding(.top, 8).padding(.bottom, 16)
                }
                .frame(maxHeight: .infinity, alignment: .top)
                VStack(spacing: 8) {
                    if working { WorkingPill() }
                    Composer(persona: persona, queued: queued, draft: draft, attachments: working ? ["look-sheet.png"] : [])
                }
                .padding(.horizontal, 12).padding(.bottom, 26)
            }
        }
        .foregroundStyle(p.fg)
    }
}

struct ChatHeader: View {
    @Environment(\.look) var look
    let persona: Persona
    var back = true
    var body: some View {
        let p = look.palette
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 10) {
                if back { RoundButton(symbol: "chevron.left", ascii: "<", size: 40) }
                Avatar(persona: persona, size: 38)
                VStack(alignment: .leading, spacing: 3) {
                    Text(persona.name).ui(16, .semibold, look)
                    AgentChip(persona: persona)
                }
                Spacer(minLength: 4)
                RoundButton(symbol: "ellipsis", ascii: "…", size: 40)
            }
            // A persona's plugins (Conductor: Queue, Events) open as sheets from their own strip (q954).
            if !persona.plugins.isEmpty {
                HStack(spacing: 8) {
                    ForEach(persona.plugins, id: \.self) { plug in
                        Text(look.mono ? "[\(plug.lowercased())]" : plug).ui(13, .semibold, look).foregroundStyle(p.g)
                            .padding(.horizontal, 12).frame(height: 30)
                            .modifier(RoundSurface(look: look))
                    }
                }
                .padding(.leading, back ? 50 : 0)
            }
        }
        .padding(.horizontal, 14).padding(.vertical, 8)
        .overlay(alignment: .bottom) { Rectangle().fill(p.line).frame(height: 0.5) }
    }
}

struct ItemView: View {
    @Environment(\.look) var look
    let item: Item
    let persona: Persona
    var body: some View {
        let p = look.palette
        switch item {
        case .you(_, let text, let files):
            VStack(alignment: .trailing, spacing: 6) {
                ForEach(files, id: \.self) { f in
                    Label(f, systemImage: "doc.text").ui(13, .regular, look).foregroundStyle(p.mute)
                        .padding(.horizontal, 10).padding(.vertical, 6).background(p.card, in: .rect(cornerRadius: 10))
                }
                Text(look.mono ? "> \(text)" : text).ui(16, .regular, look)
                    .padding(.horizontal, 14).padding(.vertical, 10)
                    .background(look.mono ? Color.clear : p.link.opacity(0.22), in: .rect(cornerRadius: 20))
                    .foregroundStyle(look.mono ? p.l : p.fg)
            }
            .frame(maxWidth: .infinity, alignment: .trailing).padding(.leading, look.mono ? 0 : 48)
        case .agent(_, let blocks):
            VStack(alignment: .leading, spacing: 10) {
                ForEach(Array(blocks.enumerated()), id: \.offset) { _, b in BlockView(block: b) }
            }
        case .tools(_, let rows):
            ToolFold(rows: rows)
        case .note(_, let text):
            Text(text).ui(13, .regular, look).foregroundStyle(p.mute).frame(maxWidth: .infinity)
        case .choice(_, let title, let detail, let options, let rec):
            ChatCard(kind: "ask", symbol: "hand.raised.fill", tint: p.r, title: title) {
                Text(detail).ui(13, .regular, look).foregroundStyle(p.mute).lineLimit(2)
                VStack(spacing: 8) {
                    ForEach(Array(options.enumerated()), id: \.offset) { i, o in
                        CardButton(title: i == rec && !look.mono ? "\(o) · recommended" : o, primary: i == rec,
                                   tint: i == options.count - 1 ? p.r : nil)
                    }
                }
            }
        case .config(_, let plugin, let title, let fields):
            ChatCard(kind: "config · \(plugin)", symbol: "slider.horizontal.3", tint: p.c, title: title) {
                ForEach(fields, id: \.0) { k, v in
                    HStack {
                        Text(k).ui(14, .regular, look).foregroundStyle(p.mute)
                        Spacer()
                        Text(v).ui(14, .regular, look)
                    }
                    .padding(.horizontal, 12).frame(height: 38).background(p.bg.opacity(0.5), in: .rect(cornerRadius: 10))
                }
                CardButton(title: "Save", primary: true, tint: p.c)
            }
        case .secret(_, let plugin, let name, let why):
            ChatCard(kind: "secret · \(plugin)", symbol: "key.fill", tint: p.a, title: name) {
                HStack {
                    Text("••••••••••••").ui(15, .regular, look).foregroundStyle(p.mute)
                    Spacer()
                    Image(systemName: "faceid").foregroundStyle(p.mute)
                }
                .padding(.horizontal, 12).frame(height: 38).background(p.bg.opacity(0.5), in: .rect(cornerRadius: 10))
                Label(why, systemImage: "lock.fill").ui(12, .regular, look).foregroundStyle(p.mute)
                CardButton(title: "Send to plugin", primary: true, tint: p.a)
            }
        case .run(_, let why, let command, let sha, let folder, let blast, let lapses):
            ChatCard(kind: "run card", symbol: "terminal.fill", tint: p.l, title: why) {
                Text("$ \(command)").font(.system(size: 13, design: .monospaced)).foregroundStyle(p.fg)
                    .padding(10).frame(maxWidth: .infinity, alignment: .leading)
                    .background(p.bg.opacity(0.6), in: .rect(cornerRadius: 10))
                VStack(alignment: .leading, spacing: 3) {
                    Text("sha \(sha)").font(.system(size: 12, design: .monospaced))
                    Text("in \(folder)")
                    Text("touches \(blast)")
                }
                .ui(12, .regular, look).foregroundStyle(p.mute)
                HStack(spacing: 8) {
                    CardButton(title: "Approve", primary: true, tint: p.g)
                    CardButton(title: "Deny", tint: p.r)
                }
                Text(lapses).ui(12, .regular, look).foregroundStyle(p.a)
            }
        }
    }
}

/// An ask, as a chat card (q954: asks are chat cards).
struct ChatCard<Content: View>: View {
    @Environment(\.look) var look
    let kind: String, symbol: String, tint: Color, title: String
    @ViewBuilder var content: Content
    var body: some View {
        let p = look.palette
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 6) {
                if !look.mono { Image(systemName: symbol).font(.system(size: 12, weight: .bold)) }
                Text(look.mono ? "## \(kind)" : kind.uppercased()).ui(11, .bold, look)
            }
            .foregroundStyle(tint)
            Text(title).ui(16, .semibold, look).foregroundStyle(p.fg)
            content
        }
        .padding(14)
        .surface(look, radius: 20)
        .overlay(alignment: .leading) {
            if look.style == .modern { RoundedRectangle(cornerRadius: 2).fill(tint).frame(width: 3).padding(.vertical, 16) }
        }
    }
}

struct BlockView: View {
    @Environment(\.look) var look
    let block: Block
    var body: some View {
        let p = look.palette
        switch block {
        case .text(let s):
            Text((try? AttributedString(markdown: s)) ?? AttributedString(s)).ui(16, .regular, look)
                .fixedSize(horizontal: false, vertical: true)
        case .bullets(let xs):
            VStack(alignment: .leading, spacing: 4) { ForEach(xs, id: \.self) { x in
                Text((try? AttributedString(markdown: "•  " + x)) ?? AttributedString(x)).ui(16, .regular, look)
            } }
        case .code(let lang, let code):
            VStack(alignment: .leading, spacing: 0) {
                HStack {
                    Text(lang).font(.system(size: 12, design: .monospaced)).foregroundStyle(p.mute)
                    Spacer()
                    Label("Copy", systemImage: "doc.on.doc").ui(12, .medium, look).foregroundStyle(p.link)
                }
                .padding(.horizontal, 12).frame(height: 32)
                .background(p.edge.opacity(0.35))
                Text(code).font(.system(size: 13, design: .monospaced)).foregroundStyle(p.fg)
                    .padding(12).frame(maxWidth: .infinity, alignment: .leading)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .background(p.card, in: .rect(cornerRadius: look.mono ? 0 : 12))
            .overlay(RoundedRectangle(cornerRadius: look.mono ? 0 : 12).strokeBorder(p.edge, lineWidth: 1))
        }
    }
}

/// Consecutive tool calls folded into one row (a tap unfolds them); the last one shows.
struct ToolFold: View {
    @Environment(\.look) var look
    let rows: [(title: String, status: String)]
    var body: some View {
        let p = look.palette, last = rows.last!, running = last.status == "running"
        HStack(spacing: 8) {
            if look.mono {
                Text(running ? "o>_" : "[ok]").ui(13, .bold, look).foregroundStyle(running ? p.c : p.g)
            } else {
                Image(systemName: running ? "gearshape.2.fill" : "checkmark.circle.fill")
                    .font(.system(size: 13)).foregroundStyle(running ? p.c : p.g)
            }
            Text(last.title).ui(13, .regular, look).foregroundStyle(p.mute).lineLimit(1).truncationMode(.middle)
            Spacer(minLength: 4)
            Text("\(rows.count) tools").ui(12, .regular, look).foregroundStyle(p.mute)
            Image(systemName: "chevron.right").font(.system(size: 10, weight: .bold)).foregroundStyle(p.mute)
        }
        .padding(.horizontal, 12).frame(height: 34)
        .background(p.card.opacity(look.mono ? 0 : 0.7), in: .rect(cornerRadius: look.mono ? 0 : 10))
        .overlay { if look.mono { Rectangle().strokeBorder(p.line, lineWidth: 1) } }
    }
}

/// The turn in progress, with stop.
struct WorkingPill: View {
    @Environment(\.look) var look
    var body: some View {
        let p = look.palette
        HStack(spacing: 8) {
            if look.mono { Text("~o=").ui(13, .bold, look).foregroundStyle(p.c) }
            else { Circle().fill(p.c).frame(width: 8, height: 8) }
            Text(look.mono ? "working 0:42" : "Working · 0:42").ui(13, .medium, look)
            Spacer()
            HStack(spacing: 4) {
                Image(systemName: "stop.fill").font(.system(size: 10))
                Text("Stop")
            }
            .ui(13, .semibold, look).foregroundStyle(p.r)
            .padding(.horizontal, 10).frame(height: 28).background(p.r.opacity(0.14), in: .capsule)
        }
        .padding(.leading, 14).padding(.trailing, 6).frame(height: 40)
        .surface(look, radius: 20)
    }
}

/// The composer: attach, the field, send; queued messages sit above it (steer or take back).
struct Composer: View {
    @Environment(\.look) var look
    let persona: Persona
    var queued: [String] = []
    var draft = ""
    var attachments: [String] = []
    var body: some View {
        let p = look.palette
        VStack(alignment: .leading, spacing: 8) {
            ForEach(queued, id: \.self) { q in
                HStack(spacing: 8) {
                    Image(systemName: "clock").font(.system(size: 12)).foregroundStyle(p.mute)
                    Text(q).ui(14, .regular, look).lineLimit(1)
                    Spacer()
                    Text("Send now").ui(12, .semibold, look).foregroundStyle(p.link)
                    Image(systemName: "xmark").font(.system(size: 11, weight: .bold)).foregroundStyle(p.mute)
                }
                .padding(.horizontal, 12).frame(height: 34)
                .background(p.card.opacity(0.8), in: .rect(cornerRadius: 12))
            }
            if !attachments.isEmpty {
                HStack(spacing: 6) {
                    ForEach(attachments, id: \.self) { a in
                        HStack(spacing: 6) {
                            Image(systemName: a.hasSuffix(".png") ? "photo" : "doc")
                            Text(a)
                            Image(systemName: "xmark.circle.fill")
                        }
                        .ui(12, .regular, look).foregroundStyle(p.mute)
                        .padding(.horizontal, 10).frame(height: 28).background(p.card, in: .capsule)
                    }
                }
            }
            HStack(alignment: .bottom, spacing: 8) {
                RoundButton(symbol: "plus", ascii: "+", size: 42)
                HStack {
                    Text(draft.isEmpty ? (queued.isEmpty ? "Message \(persona.name)" : "Queue a message") : draft)
                        .ui(16, .regular, look).foregroundStyle(draft.isEmpty ? p.mute : p.fg)
                    Spacer()
                    Image(systemName: "mic").foregroundStyle(p.mute)
                }
                .padding(.horizontal, 16).frame(height: 42)
                .surface(look, radius: 21)
                Image(systemName: "arrow.up").font(.system(size: 17, weight: .bold))
                    .foregroundStyle(p.dark ? Color.black : .white)
                    .frame(width: 42, height: 42).background(p.link, in: look.mono ? AnyShape(Rectangle()) : AnyShape(Circle()))
            }
        }
    }
}
