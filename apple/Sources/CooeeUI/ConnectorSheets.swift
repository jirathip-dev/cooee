import CooeeCore
import SwiftUI

internal struct ConnectedSheet: View {
    @Environment(\.look) private var look
    internal let label: String
    internal let account: String
    internal let personas: [Persona]
    internal var attach: (String) -> Void = { _ in }
    internal var notNow: () -> Void = {}

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 14) {
            HStack(spacing: 8) {
                Glyph(symbol: "checkmark.circle.fill", ascii: "[x]", size: 18).foregroundStyle(palette.green)
                Text("\(label) is connected as \(account)").ui(17, .semibold, look)
            }
            Text("Attach it to a persona").ui(13, .medium, look).foregroundStyle(palette.mute)
            HStack(alignment: .top, spacing: 14) {
                ForEach(personas.prefix(5)) { persona in
                    Button {
                        attach(persona.id)
                    } label: {
                        VStack(spacing: 6) {
                            FigureView(persona: persona, size: 52)
                            Text(persona.api.name).ui(11, .regular, look).lineLimit(1)
                        }
                        .frame(width: 58)
                    }
                    .buttonStyle(.plain)
                }
            }
            Button(action: notNow) {
                Text(look.label(.chip, "Not now")).ui(15, .regular, look).foregroundStyle(palette.link)
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.plain)
        }
        .bottomSheet(look)
    }
}

internal struct TokenSheet: View {
    @Environment(\.look) private var look
    internal let title: String
    internal let help: String
    @Binding internal var value: String
    internal var save: () -> Void = {}
    internal var close: () -> Void = {}

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                Text(look.label(.section, title)).ui(17, .semibold, look)
                Spacer()
                Button(action: close) { RoundButton(symbol: "xmark", ascii: "x", size: 32) }.buttonStyle(.plain)
            }
            Text(help).ui(13, .regular, look).foregroundStyle(palette.mute).fixedSize(horizontal: false, vertical: true)
            HStack {
                if look.still {
                    Text(String(repeating: "•", count: 22)).ui(17, .regular, look)
                } else {
                    SecureField("Token", text: $value).ui(17, .regular, look)
                }
                Spacer()
                if look.still {
                    ActionPill(title: "Paste")
                } else {
                    PasteButton(payloadType: String.self) { value = $0.first ?? value }.fixedSize()
                }
            }
            .padding(.horizontal, 14).frame(height: 50)
            .background(palette.background.opacity(0.6), in: .rect(cornerRadius: look.radius(12)))
            .overlay { RoundedRectangle(cornerRadius: look.radius(12)).strokeBorder(palette.link, lineWidth: 1.5) }
            HStack(alignment: .top, spacing: 8) {
                Glyph(symbol: "lock.fill", ascii: "*", size: 12).foregroundStyle(palette.green)
                Text("Goes straight to your Mac's keychain. No file, chat or log keeps it.")
                    .ui(12, .regular, look).foregroundStyle(palette.mute)
            }
            Button(action: save) { PrimaryLabel(title: "Save") }.buttonStyle(.plain)
        }
        .bottomSheet(look)
    }
}

extension View {
    fileprivate func bottomSheet(_ look: Look) -> some View {
        padding(20).padding(.bottom, 24).frame(maxWidth: .infinity, alignment: .leading)
            .chip(
                look, fill: look.palette.card, edge: look.palette.edge, radius: look.radius(28),
                paper: look.palette.background)
    }
}
