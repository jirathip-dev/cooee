import CooeeCore
@testable import CooeeUI
import SwiftUI

internal struct Mark: View {
    @Environment(\.look) private var look
    internal let connector: Connector
    internal var size: CGFloat = 34

    internal var body: some View {
        let palette = look.palette
        let tint = tint(palette)
        if look.mono {
            Text("[\(connector.mark)]").ui(size * 0.42, .bold, look).foregroundStyle(tint).frame(width: size)
        } else {
            Text(connector.mark).font(.system(size: size * 0.5, weight: .bold, design: .rounded))
                .foregroundStyle(.white).frame(width: size, height: size)
                .background(tint.gradient, in: .rect(cornerRadius: size * 0.26))
        }
    }

    private func tint(_ palette: Palette) -> Color {
        switch connector.source {
        case .command: palette.mute
        case .typed: palette.amber
        case .catalogue: connector.name == "linear" || connector.name == "github" ? palette.purple : palette.cyan
        }
    }
}

internal struct Provenance: View {
    @Environment(\.look) private var look
    internal let connector: Connector

    internal var body: some View {
        let palette = look.palette
        switch connector.source {
        case .catalogue:
            HStack(spacing: 4) {
                Glyph(symbol: "checkmark.seal.fill", ascii: "✓", size: 11).foregroundStyle(palette.green)
                Text("reviewed · run by \(connector.publisher)").ui(12, .regular, look).foregroundStyle(palette.mute)
            }
        case .typed:
            HStack(spacing: 4) {
                Glyph(symbol: "exclamationmark.triangle.fill", ascii: "!", size: 11)
                Text("not reviewed · \(connector.host)").ui(12, .medium, look)
            }
            .foregroundStyle(palette.amber)
        case .command:
            Text("a command on this Mac · added on the Mac").ui(12, .regular, look).foregroundStyle(palette.mute)
        }
    }
}

internal struct AccountWord: View {
    @Environment(\.look) private var look
    internal let account: Account

    internal var body: some View {
        let palette = look.palette
        switch account.state {
        case .signedIn: StateWord(text: "\(account.name) · signed in", tint: palette.green)
        case .needsSignIn: StateWord(text: "\(account.name) · needs sign-in", tint: palette.amber)
        case .signingIn: StateWord(text: "\(account.name) · signing in…", tint: palette.link)
        }
    }
}

internal struct ToolCounts: View {
    @Environment(\.look) private var look
    internal let connector: Connector

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 10) {
            count(connector.read.count, "run", palette.green)
            if !connector.write.isEmpty { count(connector.write.count, "ask first", palette.amber) }
            if !connector.never.isEmpty { count(connector.never.count, "never", palette.red) }
        }
    }

    private func count(_ number: Int, _ word: String, _ tint: Color) -> some View {
        HStack(spacing: 4) {
            Text("\(number)").ui(12, .semibold, look).foregroundStyle(tint)
            Text(word).ui(12, .regular, look).foregroundStyle(look.palette.mute)
        }
    }
}

internal struct Chevron: View {
    @Environment(\.look) private var look

    internal var body: some View {
        Glyph(symbol: "chevron.right", ascii: ">", size: 12).foregroundStyle(look.palette.mute)
    }
}

internal struct Dim: View {
    @Environment(\.look) private var look

    internal var body: some View {
        Color.black.opacity(look.palette.dark ? 0.5 : 0.28).padding(.top, -54)
    }
}

internal struct Primary: View {
    @Environment(\.look) private var look
    internal let title: String
    internal var symbol = ""

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 8) {
            if !symbol.isEmpty && !look.mono { Image(systemName: symbol).font(.system(size: 15, weight: .semibold)) }
            Text(look.mono ? "[\(title.lowercased())]" : title).ui(16, .semibold, look)
        }
        .foregroundStyle(look.mono ? palette.link : (palette.dark ? Color.black : .white))
        .frame(maxWidth: .infinity).frame(height: 48)
        .background(look.mono ? Color.clear : palette.link, in: .rect(cornerRadius: 14))
        .overlay { if look.mono { Rectangle().strokeBorder(palette.link, lineWidth: 1) } }
    }
}
