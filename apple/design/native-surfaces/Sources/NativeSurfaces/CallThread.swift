import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct CallThread: View {
    @Environment(\.look) private var look
    private let researcher = Persona.named("res", .done)

    private static let after: [Item] = [
        .tools(id: 2, rows: [ToolRow("fetch sec.gov/…/kestrel-10q-q3.htm", "ok"), ToolRow("read notes, p. 14", "ok")]),
        .agent(
            id: 3,
            markdown: [
                .text("As I said on the call: Birch fell to **14%** because its renewal is deferred to January."),
                .bullets(["Source: Q3 10-Q, notes p. 14", "Saved to `notes/kestrel.md`"]),
            ]),
    ]

    public var body: some View {
        ZStack {
            Backdrop()
            VStack(spacing: 0) {
                header
                VStack(alignment: .leading, spacing: 14) {
                    ItemView(item: .you(id: 1, text: "Find what the Q3 10-Q says about Birch."))
                    CallBlock()
                    ForEach(Self.after) { ItemView(item: $0) }
                }
                .padding(.horizontal, 16).padding(.top, 8)
                Spacer()
                Composer(hint: "Message \(researcher.name)").padding(.horizontal, 12).padding(.bottom, 26)
            }
        }
        .foregroundStyle(look.palette.foreground)
    }

    private var header: some View {
        HStack(spacing: 10) {
            RoundButton(symbol: "chevron.left", ascii: "<", size: 40)
            Avatar(persona: researcher, size: 38)
            VStack(alignment: .leading, spacing: 3) {
                Text(researcher.name).ui(16, .semibold, look)
                AgentChip(persona: researcher)
            }
            Spacer(minLength: 4)
            RoundButton(symbol: "phone.fill", ascii: "c", size: 40)
            RoundButton(symbol: "ellipsis", ascii: "…", size: 40)
        }
        .padding(.horizontal, 14).padding(.vertical, 8)
        .overlay(alignment: .bottom) { Rectangle().fill(look.palette.line).frame(height: 0.5) }
    }

    public init() {}
}

internal struct CallBlock: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 6) {
                Image(systemName: "phone.fill").foregroundStyle(palette.green)
                Text(look.mono ? "[call] 4:12 · 6 lines · 2 tools" : "Call · 4:12 · 6 lines · 2 tools")
                    .ui(13, .semibold, look)
            }
            ForEach(Spoken.lines) { line in
                HStack(alignment: .top, spacing: 6) {
                    Text(line.you ? "You" : "Researcher").ui(13, .semibold, look)
                        .foregroundStyle(line.you ? palette.mute : palette.purple).frame(width: 78, alignment: .leading)
                    Text(line.text).ui(13, .regular, look).fixedSize(horizontal: false, vertical: true)
                }
            }
            Text("Show all 6 lines").ui(13, .semibold, look).foregroundStyle(palette.link)
        }
        .padding(14).frame(maxWidth: .infinity, alignment: .leading).surface(look, radius: 18)
    }
}
