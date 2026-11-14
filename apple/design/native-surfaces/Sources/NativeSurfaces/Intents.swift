import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct SiriSnippet: View {
    @Environment(\.look) private var look

    public var body: some View {
        let palette = look.palette
        ZStack {
            Wallpaper(dim: true)
            VStack(spacing: 14) {
                card
                Spacer()
                Text("“Tell Coder in cooee to rerun the Kestrel export with the Q3 filing”")
                    .ui(17, .medium, look).multilineTextAlignment(.center).foregroundStyle(.white)
                    .padding(.horizontal, 30)
                glow.frame(height: 90)
            }
            .padding(.top, 20).foregroundStyle(palette.foreground)
        }
    }

    private var card: some View {
        let palette = look.palette
        let coder = Persona.named("coder", .idle)
        return VStack(alignment: .leading, spacing: 12) {
            Kicker(text: "cooee")
            Text("Send to Coder?").ui(22, .bold, look)
            Text("Export for Kestrel · studio · the latest thread").ui(13, .regular, look)
                .foregroundStyle(palette.mute)
            HStack(alignment: .top, spacing: 10) {
                Avatar(persona: coder, size: 34)
                Text("Rerun the Kestrel export with the Q3 filing.").ui(16, .regular, look)
                    .padding(12).frame(maxWidth: .infinity, alignment: .leading)
                    .background(palette.card, in: .rect(cornerRadius: look.mono ? 0 : 16))
            }
            HStack(spacing: 10) {
                CardButton(title: "Edit in cooee")
                CardButton(title: "Send", primary: true)
            }
            Text("Coder starts at once; the island follows the turn.").ui(12, .regular, look)
                .foregroundStyle(palette.mute)
        }
        .padding(18).surface(look, radius: 30).padding(.horizontal, 10)
    }

    @ViewBuilder private var glow: some View {
        let palette = look.palette
        if look.mono {
            Text("~o= listening").ui(15, .bold, look).foregroundStyle(palette.cyan)
        } else {
            Capsule()
                .fill(
                    AngularGradient(
                        colors: [palette.cyan, palette.purple, palette.red, palette.amber, palette.cyan],
                        center: .center)
                )
                .frame(width: 300, height: 14).blur(radius: 10)
        }
    }

    public init() {}
}

internal struct ShortcutTile: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal let symbol: String
    internal let title: String

    internal var body: some View {
        let tint = look.palette.tint(persona.look.tint)
        VStack(alignment: .leading, spacing: 0) {
            Image(systemName: symbol).font(.system(size: 20, weight: .semibold))
            Spacer()
            Text(title).ui(15, .semibold, look).lineLimit(2).multilineTextAlignment(.leading)
        }
        .foregroundStyle(look.palette.dark ? Color.black.opacity(0.8) : .white)
        .padding(12).frame(width: 175, height: 96, alignment: .topLeading)
        .background(look.mono ? tint.opacity(0.85) : tint, in: .rect(cornerRadius: look.mono ? 0 : 20))
    }
}

public struct ShortcutsScreen: View {
    @Environment(\.look) private var look

    private static let tiles: [(String, String, String)] = [
        ("chief", "plus.bubble.fill", "New chat with Chief"),
        ("coder", "waveform", "Tell Coder…"),
        ("res", "phone.fill", "Call Researcher"),
        ("cond", "exclamationmark.bubble.fill", "What needs me"),
        ("coder", "stop.fill", "Stop the running turn"),
        ("am", "text.bubble.fill", "Open Birch & Co pricing"),
    ]

    public var body: some View {
        let palette = look.palette
        ZStack(alignment: .top) {
            Backdrop()
            VStack(alignment: .leading, spacing: 14) {
                Text("cooee").ui(32, .bold, look)
                Kicker(text: "App Shortcuts · ready, no setup")
                LazyVGrid(columns: [GridItem(.fixed(175)), GridItem(.fixed(175))], spacing: 12) {
                    ForEach(Array(Self.tiles.enumerated()), id: \.offset) { _, tile in
                        ShortcutTile(persona: .named(tile.0), symbol: tile.1, title: tile.2)
                    }
                }
                ActionButtonCard()
                Kicker(text: "Say to Siri")
                Text("“New chat with Chief in cooee” · “Tell Coder in cooee …” · “What needs me in cooee”")
                    .ui(14, .regular, look).foregroundStyle(palette.mute)
            }
            .padding(.horizontal, 18).padding(.top, 12).foregroundStyle(palette.foreground)
        }
    }

    public init() {}
}

internal struct ActionButtonCard: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 14) {
            Image(systemName: look.mono ? "button.horizontal" : "button.horizontal.top.press.fill")
                .font(.system(size: 26)).foregroundStyle(palette.amber).frame(width: 40)
            VStack(alignment: .leading, spacing: 3) {
                Text("Action button: What needs me").ui(15, .semibold, look)
                Text(
                    "Press opens Needs you in cooee: asks, then blocked, then running. Any shortcut above can take its place."
                )
                .ui(13, .regular, look).foregroundStyle(palette.mute)
                .fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(14).frame(maxWidth: .infinity, alignment: .leading).surface(look, radius: 22)
    }
}
