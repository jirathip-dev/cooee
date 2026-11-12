import CooeeCore
@testable import CooeeUI
import SwiftUI

internal struct Command: Sendable {
    internal let name: String
    internal var hint = ""
    internal let detail: String

    internal static let cooee: [Command] = [
        Command(name: "new", detail: "New chat with Coder"),
        Command(name: "stop", detail: "Stop the running turn"),
        Command(name: "model", hint: "<model>", detail: "Switch the model · gpt-5.5"),
    ]

    internal static let agent: [Command] = [
        Command(name: "review", hint: "[focus]", detail: "Review the working tree"),
        Command(name: "compact", detail: "Summarize the chat to free context"),
        Command(name: "mention", hint: "<file>", detail: "Point the agent at a file"),
        Command(name: "memory", detail: "Show what the agent keeps"),
        Command(name: "diff", detail: "Show the uncommitted diff"),
        Command(name: "status", detail: "Session, model and limits"),
    ]

    internal static func matching(_ prefix: String) -> [(Command, String)] {
        cooee.filter { $0.name.hasPrefix(prefix) }.map { ($0, "cooee") }
            + agent.filter { $0.name.hasPrefix(prefix) }.map { ($0, "Coder") }
    }
}

internal struct CommandRow: View {
    @Environment(\.look) private var look
    internal let command: Command
    internal var typed = ""
    internal var source: String?

    internal var body: some View {
        let palette = look.palette
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    name
                    if !command.hint.isEmpty {
                        Text(command.hint).font(.system(size: 14, design: .monospaced)).foregroundStyle(palette.mute)
                    }
                }
                Text(command.detail).ui(13, .regular, look).foregroundStyle(palette.mute)
            }
            Spacer()
            if let source {
                Text(look.mono ? "[\(source.lowercased())]" : source).ui(11, .semibold, look)
                    .foregroundStyle(source == "cooee" ? palette.lead : palette.cyan)
            }
            Glyph(symbol: command.hint.isEmpty ? "return" : "text.cursor", ascii: command.hint.isEmpty ? "↵" : "…", size: 13)
                .foregroundStyle(palette.mute).frame(width: 18)
        }
        .padding(.horizontal, 14).padding(.vertical, 9)
    }

    private var name: some View {
        let palette = look.palette
        let rest = String(command.name.dropFirst(max(0, typed.count - 1)))
        return (Text(typed.isEmpty ? "/" : typed).foregroundStyle(palette.link).bold()
            + Text(typed.isEmpty ? command.name : rest))
            .font(.system(size: 16, weight: .medium, design: .monospaced))
    }
}

internal struct SlashList: View {
    @Environment(\.look) private var look
    internal let typed: String

    internal var body: some View {
        let palette = look.palette
        let rows = Command.matching(String(typed.dropFirst()))
        VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(rows.enumerated()), id: \.offset) { index, row in
                CommandRow(command: row.0, typed: typed, source: row.1)
                    .background(index == 0 ? palette.link.opacity(look.mono ? 0.16 : 0.12) : .clear)
                if index < rows.count - 1 { Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, 14) }
            }
        }
        .padding(.vertical, 4)
        .surface(look, radius: 18)
        .clipShape(.rect(cornerRadius: look.mono ? 0 : 18))
    }
}

internal struct CommandsSheet: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        Sheet {
            VStack(alignment: .leading, spacing: 10) {
                Text(look.mono ? "# commands" : "Commands").ui(17, .semibold, look).frame(maxWidth: .infinity)
                HStack(spacing: 8) {
                    Glyph(symbol: "magnifyingglass", ascii: "/", size: 14).foregroundStyle(palette.mute)
                    Text(look.mono ? "filter_" : "Search commands").ui(15, .regular, look).foregroundStyle(palette.mute)
                    Spacer()
                }
                .padding(.horizontal, 12).frame(height: 38).surface(look, radius: 12)
                section("cooee", Command.cooee)
                section("Coder's agent", Command.agent)
            }
            .padding(.horizontal, 14).padding(.bottom, 30)
        }
    }

    private func section(_ title: String, _ commands: [Command]) -> some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 4) {
            Text(look.mono ? "# \(title.lowercased())" : title.uppercased()).ui(11, .bold, look)
                .foregroundStyle(palette.mute).padding(.leading, 6).padding(.top, 6)
            VStack(spacing: 0) {
                ForEach(Array(commands.enumerated()), id: \.offset) { index, command in
                    CommandRow(command: command)
                    if index < commands.count - 1 {
                        Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, 14)
                    }
                }
            }
            .background(look.mono ? Color.clear : palette.card.opacity(0.7), in: .rect(cornerRadius: 14))
            .overlay { if look.mono { Rectangle().strokeBorder(palette.edge, style: StrokeStyle(lineWidth: 1, dash: [3, 2])) } }
        }
    }
}
