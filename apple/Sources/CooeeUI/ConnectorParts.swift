import CooeeCore
import SwiftUI

internal struct ConnectorMark: View {
    @Environment(\.look) private var look
    internal let connector: Connector
    internal var size: CGFloat = 34

    internal var body: some View {
        let letter = String(connector.label.prefix(1))
        Mark(text: "[\(letter)]", size: size * 0.42) {
            Text(letter).font(.system(size: size * 0.5, weight: .bold, design: .rounded))
                .foregroundStyle(.white).frame(width: size, height: size)
                .background(tint.gradient, in: .rect(cornerRadius: size * 0.26))
        }
        .foregroundStyle(tint).frame(width: size)
    }

    private var tint: Color {
        let palette = look.palette
        switch connector.source {
        case .command: return palette.mute
        case .typed: return palette.amber
        case .catalogue:
            let tints = [palette.link, palette.green, palette.cyan, palette.purple, palette.lead, palette.link]
            return (tints + [palette.green, palette.cyan])[PersonaLook.seed(connector.name) % 8]
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
    internal let account: Connector.Account

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
    internal let tools: Connector.Tools

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 10) {
            count(tools.read.count, "run", palette.green)
            if !tools.write.isEmpty { count(tools.write.count, "ask first", palette.amber) }
            if !tools.never.isEmpty { count(tools.never.count, "never", palette.red) }
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

internal struct PrimaryLabel: View {
    @Environment(\.look) private var look
    internal let title: String
    internal var symbol = ""

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 8) {
            if !symbol.isEmpty && !look.textMarks {
                Image(systemName: symbol).font(.system(size: 15, weight: .semibold))
            }
            Text(look.label(.chip, title)).ui(16, .semibold, look)
        }
        .foregroundStyle(look.outline ? palette.link : (palette.dark ? Color.black : .white))
        .frame(maxWidth: .infinity).frame(height: 48)
        .chip(look, fill: palette.link, edge: palette.link, radius: 14)
    }
}

extension Connector {
    internal static func day(_ style: Date.FormatStyle = .init(timeZone: .current)) -> Date.FormatStyle {
        var style = style
        style.locale = Locale(identifier: "en_GB")
        style.calendar = Calendar(identifier: .gregorian)
        return style.day().month(.abbreviated)
    }

    internal var checkedDay: String {
        let day = (try? Date(checked ?? "", strategy: .iso8601.year().month().day()))
        return day?.formatted(Self.day(Date.FormatStyle(timeZone: .gmt)).year()) ?? ""
    }
}

extension Connector.Account {
    internal func day(now: Date = Date()) -> String {
        let date = Date(timeIntervalSince1970: Double(since) / 1000)
        return now.timeIntervalSince(date) < 60 ? "just now" : date.formatted(Connector.day())
    }
}
