import CooeeCore
import SwiftUI

/// Siri's confirmation before a dictated message is sent: to whom, which thread, and the words.
public struct SendSnippet: View {
    private let card: SnippetCard
    private let style: Style?

    public var body: some View { card.surfaceLook(style) }

    nonisolated public init(persona: Persona, place: String, text: String, style: Style?) {
        card = SnippetCard(persona: persona, place: place, text: text)
        self.style = style
    }
}

private struct SnippetCard: View {
    @Environment(\.look) private var look
    private let persona: Persona
    private let place: String
    private let text: String

    nonisolated fileprivate init(persona: Persona, place: String, text: String) {
        self.persona = persona
        self.place = place
        self.text = text
    }

    fileprivate var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 12) {
            Kicker(text: "cooee")
            Text("Send to \(persona.name)?").ui(22, .bold, look)
            Text(place).ui(13, .regular, look).foregroundStyle(palette.mute)
            HStack(alignment: .top, spacing: 10) {
                FigureView(persona: persona, size: 34)
                Text(text).ui(16, .regular, look).padding(12).frame(maxWidth: .infinity, alignment: .leading)
                    .background(palette.card, in: .rect(cornerRadius: look.type == .mono ? 0 : 16))
            }
        }
        .padding(18).foregroundStyle(palette.foreground)
    }
}

private struct SurfaceLook: ViewModifier {
    @Environment(\.colorScheme) private var scheme
    fileprivate let style: Style?

    fileprivate func body(content: Content) -> some View {
        let palette: Palette = scheme == .dark ? .farmDark : .farmLight
        return content.environment(\.look, Look(palette: palette, style: style ?? .bundled, still: true))
    }
}

extension View {
    /// The look a widget, the Live Activity or a snippet draws in: the farm palette for the system's mode, and the
    /// app's style (none: Modern).
    public func surfaceLook(_ style: Style?) -> some View { modifier(SurfaceLook(style: style)) }
}
