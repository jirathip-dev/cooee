import CooeeCore
import SwiftUI

private struct AskerKey: EnvironmentKey {
    fileprivate static var defaultValue: Persona? { nil }
}

private struct AskDepthKey: EnvironmentKey {
    fileprivate static var defaultValue: Int? { nil }
}

private struct CastKey: EnvironmentKey {
    fileprivate static var defaultValue: [Persona] { Fixtures.personas }
}

private struct EditPersonaKey: EnvironmentKey {
    fileprivate static var defaultValue: @MainActor (String) -> Void { { _ in } }
}

extension EnvironmentValues {
    internal var asker: Persona? {
        get { self[AskerKey.self] }
        set { self[AskerKey.self] = newValue }
    }

    internal var askDepth: Int? {
        get { self[AskDepthKey.self] }
        set { self[AskDepthKey.self] = newValue }
    }

    internal var cast: [Persona] {
        get { self[CastKey.self] }
        set { self[CastKey.self] = newValue }
    }

    internal var editPersona: @MainActor (String) -> Void {
        get { self[EditPersonaKey.self] }
        set { self[EditPersonaKey.self] = newValue }
    }
}

internal struct AskCard: View {
    @Environment(\.look) private var look
    @Environment(\.asker) private var asker
    @Environment(\.askDepth) private var depth
    @Environment(\.cast) private var cast
    @Environment(\.openThread) private var openThread
    @Environment(\.editPersona) private var editPersona
    internal let ask: Ask

    internal var body: some View {
        let palette = look.palette
        let from = (asker ?? Persona(APIPersona(id: "", name: "", agent: ""))).with(state: .done)
        let target =
            (cast.first { $0.id == ask.persona } ?? Persona(APIPersona(id: ask.persona, name: ask.name, agent: "")))
            .with(state: .done)
        let edge = ask.refused ? palette.red.opacity(0.55) : palette.edge
        VStack(alignment: .leading, spacing: 8) {
            head(from, target)
            Text(ask.text).ui(15, .regular, look).lineLimit(2).fixedSize(horizontal: false, vertical: true)
            if !ask.note.isEmpty { note }
            outcome(from)
        }
        .padding(.horizontal, 12).padding(.vertical, 10)
        .background(look.outline ? .clear : palette.card.opacity(0.7), in: .rect(cornerRadius: 16))
        .overlay {
            if look.outline {
                Rectangle().fill(palette.link).frame(width: 3).frame(maxWidth: .infinity, alignment: .leading)
            } else {
                RoundedRectangle(cornerRadius: 16).strokeBorder(edge, lineWidth: 1)
            }
        }
    }

    private func head(_ from: Persona, _ target: Persona) -> some View {
        let palette = look.palette
        let asked = Text(" asked ").foregroundStyle(palette.mute)
        return HStack(spacing: 8) {
            FigureView(persona: from, size: 22)
            Text("→").ui(13, .regular, look).foregroundStyle(palette.mute)
            FigureView(persona: target, size: 22)
            Text("\(Text(from.name).fontWeight(.semibold))\(asked)\(Text(target.name).fontWeight(.semibold))")
                .ui(15, .regular, look).lineLimit(1)
        }
    }

    private var note: some View {
        let palette = look.palette
        return Text("\(Text("Your note").fontWeight(.semibold).foregroundStyle(palette.mute))  \(ask.note)")
            .ui(14, .regular, look).lineLimit(2).padding(.horizontal, 10).padding(.vertical, 6)
            .frame(maxWidth: .infinity, alignment: .leading)
            .accent(look, palette.link, fill: palette.link.opacity(0.1), radius: 12)
    }

    private func outcome(_ from: Persona) -> some View {
        let palette = look.palette
        return HStack(spacing: 8) {
            Text(ask.line(asker: from.name, depth: depth)).ui(15, .regular, look).lineLimit(1)
                .foregroundStyle(ask.refused ? palette.red : palette.foreground)
            Spacer(minLength: 4)
            if ask.refused {
                link("Persona ›") { editPersona(from.id) }
            } else if !ask.thread.isEmpty {
                link("Open ›") { openThread(ask.thread) }
            }
        }
    }

    private func link(_ title: String, _ action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title).ui(15, .semibold, look).foregroundStyle(look.palette.link).fixedSize()
        }
        .buttonStyle(.plain)
    }
}
