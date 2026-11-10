import SwiftUI

/// Grouped rows, inset like iOS Settings (Terminal: ruled, no rounding).
struct Group2<Content: View>: View {
    @Environment(\.look) var look
    var header = ""
    @ViewBuilder var content: Content
    var body: some View {
        let p = look.palette
        VStack(alignment: .leading, spacing: 6) {
            if !header.isEmpty {
                Text(look.mono ? "# \(header.lowercased())" : header.uppercased()).ui(12, .medium, look)
                    .foregroundStyle(p.mute).padding(.leading, 16)
            }
            VStack(alignment: .leading, spacing: 0) { content }
                .background(p.card.opacity(look.style == .modern ? 0.75 : 1), in: .rect(cornerRadius: look.mono ? 0 : 16))
                .overlay { if look.mono { Rectangle().strokeBorder(p.edge, lineWidth: 1) } }
        }
    }
}

struct SettingRow: View {
    @Environment(\.look) var look
    let symbol: String, title: String
    var tint: Color? = nil
    var value = ""
    var divider = true
    var body: some View {
        let p = look.palette
        HStack(spacing: 12) {
            if !look.mono {
                Image(systemName: symbol).font(.system(size: 14, weight: .semibold)).foregroundStyle(.white)
                    .frame(width: 30, height: 30).background(tint ?? p.link, in: .rect(cornerRadius: 8))
            }
            Text(title).ui(16, .regular, look)
            Spacer()
            Text(value).ui(15, .regular, look).foregroundStyle(p.mute).lineLimit(1)
            Image(systemName: "chevron.right").font(.system(size: 12, weight: .semibold)).foregroundStyle(p.mute)
        }
        .padding(.horizontal, 14).frame(height: 48)
        .overlay(alignment: .bottom) {
            if divider { Rectangle().fill(p.line).frame(height: 0.5).padding(.leading, look.mono ? 0 : 56) }
        }
    }
}

/// The avatar menu (q954): who you are and where, then Settings (Look, Notifications, Sign-ins, Devices), Machines
/// and Plugins. Their own screens come in v2 (q955); here they are rows.
public struct AvatarMenu: View {
    @Environment(\.look) var look
    public init() {}
    public var body: some View {
        let p = look.palette
        VStack(alignment: .leading, spacing: 14) {
            HStack(spacing: 12) {
                AccountAvatar(size: 48)
                VStack(alignment: .leading, spacing: 2) {
                    Text("Guy").ui(18, .semibold, look)
                    Text("this iPhone · paired to mini").ui(13, .regular, look).foregroundStyle(p.mute)
                }
                Spacer()
            }
            .padding(.horizontal, 6)
            Group2(header: "Settings") {
                SettingRow(symbol: "paintpalette.fill", title: "Look", tint: p.a, value: "Modern · Farm")
                SettingRow(symbol: "bell.badge.fill", title: "Notifications", tint: p.r, value: "Asks, done")
                SettingRow(symbol: "person.badge.key.fill", title: "Sign-ins", tint: p.p, value: "Claude, Codex")
                SettingRow(symbol: "iphone.gen3", title: "Devices", tint: p.link, value: "3", divider: false)
            }
            Group2 {
                SettingRow(symbol: "desktopcomputer", title: "Machines", tint: p.c, value: "mini, air · Terminal")
                SettingRow(symbol: "puzzlepiece.extension.fill", title: "Plugins", tint: p.g, value: "4", divider: false)
            }
            Text(look.mono ? "-- v2: these open their screens" : "Machines, Terminal and the plugin sheets open in v2.")
                .ui(12, .regular, look).foregroundStyle(p.mute).padding(.leading, 6)
        }
        .padding(14)
        .surface(look, radius: 30)
        .foregroundStyle(p.fg)
    }
}

/// The persona sheet: avatar, name, the agent and model chip, Character, Instructions, memory, folder.
public struct PersonaSheet: View {
    @Environment(\.look) var look
    let persona: Persona
    public init(personaID: String = "chief") { persona = Fixtures.persona(personaID) }
    public var body: some View {
        let p = look.palette
        ZStack(alignment: .top) {
            // The chat dimmed behind the sheet's top edge.
            p.bg.opacity(0.4).ignoresSafeArea()
            VStack(spacing: 0) {
                Capsule().fill(p.mute.opacity(0.5)).frame(width: 36, height: 5).padding(.top, 6)
                HStack {
                    RoundButton(symbol: "xmark", ascii: "x", size: 36)
                    Spacer()
                    Text(persona.name).ui(17, .semibold, look)
                    Spacer()
                    RoundButton(symbol: "checkmark", ascii: "ok", size: 36)
                }
                .padding(.horizontal, 16).padding(.vertical, 8)
                Scroller {
                    VStack(spacing: 18) {
                        VStack(spacing: 8) {
                            Avatar(persona: persona, size: 92)
                            Text("Edit avatar").ui(14, .medium, look).foregroundStyle(p.link)
                        }
                        Group2 {
                            field("Name", persona.name)
                            HStack {
                                Text("Agent").ui(16, .regular, look)
                                Spacer()
                                AgentChip(persona: persona, effort: true)
                            }
                            .padding(.horizontal, 14).frame(height: 48)
                        }
                        Group2(header: "Character") { para(persona.character) }
                        Group2(header: "Instructions") { para(persona.instructions) }
                        Group2(header: "Memory · 12 notes") {
                            ForEach(Fixtures.chiefMemory, id: \.0) { note, from in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(note).ui(15, .regular, look).lineLimit(1)
                                    Text(from).ui(12, .regular, look).foregroundStyle(p.mute)
                                }
                                .padding(.horizontal, 14).padding(.vertical, 9)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .overlay(alignment: .bottom) { Rectangle().fill(p.line).frame(height: 0.5).padding(.leading, 14) }
                            }
                            HStack {
                                Text("Show all 12").foregroundStyle(p.link)
                                Spacer()
                                Text("Clear memory…").foregroundStyle(p.r)
                            }
                            .ui(15, .regular, look).padding(.horizontal, 14).frame(height: 44)
                        }
                        Group2 { SettingRow(symbol: "folder.fill", title: "Folder", tint: p.a, value: persona.folder, divider: false) }
                    }
                    .padding(.horizontal, 16).padding(.bottom, 30)
                }
            }
            .background(
                UnevenRoundedRectangle(topLeadingRadius: 34, topTrailingRadius: 34).fill(p.bg)
                    .overlay(UnevenRoundedRectangle(topLeadingRadius: 34, topTrailingRadius: 34).fill(p.card.opacity(0.35)))
                    .ignoresSafeArea())
            .padding(.top, 10)
        }
        .foregroundStyle(p.fg)
    }

    func field(_ k: String, _ v: String) -> some View {
        HStack {
            Text(k).ui(16, .regular, look)
            Spacer()
            Text(v).ui(16, .regular, look).foregroundStyle(look.palette.mute)
        }
        .padding(.horizontal, 14).frame(height: 48)
        .overlay(alignment: .bottom) { Rectangle().fill(look.palette.line).frame(height: 0.5).padding(.leading, 14) }
    }
    func para(_ s: String) -> some View {
        Text(s).ui(15, .regular, look).padding(14).frame(maxWidth: .infinity, alignment: .leading)
            .fixedSize(horizontal: false, vertical: true)
    }
}
