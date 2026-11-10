import CooeeCore
import SwiftUI

internal struct PinnedRow: View {
    @Environment(\.look) private var look
    internal let personas: [Persona]
    internal let rows: [Row]
    internal var new: (String) -> Void = { _ in }

    private var scene: Bool { look.style == .scene }

    internal var body: some View {
        switch look.style {
        case .scene:
            ZStack(alignment: .bottom) {
                Sprite(name: "scene-strip").scaledToFill().frame(height: 180).clipped()
                strip.padding(.bottom, 8)
            }
            .frame(height: 180)
        case .terminal:
            VStack(alignment: .leading, spacing: 8) {
                Text("# pinned").ui(12, .regular, look).foregroundStyle(look.palette.mute).padding(.horizontal, 16)
                strip
            }
        case .modern:
            strip
        }
    }

    private var strip: some View {
        Scroller(axis: .horizontal) {
            HStack(alignment: .bottom, spacing: scene ? 4 : 14) {
                ForEach(personas) { persona in
                    Button {
                        new(persona.id)
                    } label: {
                        pin(persona)
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 16)
        }
        .frame(height: look.style == .modern ? 96 : 116)
    }

    private func pin(_ persona: Persona) -> some View {
        let palette = look.palette
        let ink = scene ? Color.hex(0x29251a) : palette.foreground
        return VStack(spacing: 6) {
            ZStack(alignment: .topTrailing) {
                Avatar(persona: persona, size: scene ? 58 : 62)
                if let row = rows.first(where: { $0.persona == persona.id && $0.unseen > 0 }) {
                    Badge(count: row.unseen).offset(x: 4, y: -2)
                }
            }
            Text(persona.name).ui(12, .medium, look).lineLimit(1).foregroundStyle(ink)
                .padding(.horizontal, scene ? 6 : 0).padding(.vertical, scene ? 2 : 0)
                .background(scene ? Color.hex(0xf5efda).opacity(0.85) : .clear, in: .capsule)
            if look.style != .modern {
                Text(persona.word).ui(10, .regular, look)
                    .foregroundStyle(
                        persona.state == .needs ? palette.red : (scene ? Color.hex(0x29251a) : palette.mute))
            }
        }
        .frame(width: scene ? 78 : 70)
    }
}

internal struct ChatRow: View {
    @Environment(\.look) private var look
    internal let row: Row
    internal let persona: Persona
    internal var selected = false

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 12) {
            Avatar(persona: persona.with(state: row.state), size: 48)
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    Text(persona.name).ui(16, .semibold, look).lineLimit(1)
                    Text(row.title).ui(14, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
                    Spacer(minLength: 4)
                    Text(row.ago).ui(13, .regular, look).foregroundStyle(row.unseen > 0 ? palette.link : palette.mute)
                }
                detail
            }
        }
        .padding(.horizontal, 16).padding(.vertical, 10)
        .background(selected ? palette.link.opacity(0.16) : .clear, in: .rect(cornerRadius: look.mono ? 0 : 14))
        .padding(.horizontal, selected ? 6 : 0)
        .contentShape(.rect)
    }

    private var detail: some View {
        let palette = look.palette
        return HStack(spacing: 6) {
            if row.state == .needs {
                if look.mono {
                    Text("!").ui(14, .bold, look).foregroundStyle(palette.red)
                } else {
                    Image(systemName: "exclamationmark.circle.fill").font(.system(size: 13)).foregroundStyle(
                        palette.red)
                }
            }
            if row.state == .working && !look.mono {
                Image(systemName: "ellipsis").font(.system(size: 13, weight: .bold)).foregroundStyle(palette.cyan)
            }
            Text(row.last).ui(14, .regular, look).foregroundStyle(palette.mute).lineLimit(1)
            Spacer(minLength: 4)
            if row.unseen > 0 { Badge(count: row.unseen) }
        }
    }
}
