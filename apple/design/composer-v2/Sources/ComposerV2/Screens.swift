import CooeeCore
@testable import CooeeUI
import SwiftUI

public enum Over: Sendable, Equatable {
    case none, slash, menu
    case attach(AttachTab)
}

public struct ChatScreen: View {
    @Environment(\.look) private var look
    private let bar: BarState
    private let keyboard: Keyboard
    private let over: Over
    private let voice: Bool
    private let note: String?

    public var body: some View {
        ZStack(alignment: .bottom) {
            Backdrop()
            VStack(spacing: 0) {
                ChatHeader(persona: Story.coder, chips: [])
                Scroller(end: true) { ThreadView(voice: voice) }.frame(maxHeight: .infinity)
                VStack(spacing: 8) {
                    if let note { Toast(text: note) }
                    if over == .slash { SlashList(typed: "/m") }
                    ComposerBar(state: bar)
                }
                .padding(.horizontal, look.mono ? 8 : 12).padding(.bottom, keyboard == .none ? 30 : 8)
                if keyboard != .none { KeyboardView(kind: keyboard) }
            }
            if over == .menu || isAttach {
                Dim()
                if case .attach(let tab) = over { AttachSheet(tab: tab) } else { CommandsSheet() }
            }
        }
        .foregroundStyle(look.palette.foreground)
    }

    private var isAttach: Bool { if case .attach = over { true } else { false } }

    public init(
        bar: BarState = .empty, keyboard: Keyboard = .none, over: Over = .none, voice: Bool = false, note: String? = nil
    ) {
        self.bar = bar
        self.keyboard = keyboard
        self.over = over
        self.voice = voice
        self.note = note
    }
}

internal struct ThreadView: View {
    internal let voice: Bool

    internal var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            ForEach(Story.items) { ItemView(item: $0) }
            if voice {
                VoiceBubble()
                ItemView(item: Story.reply)
            }
        }
        .padding(.horizontal, 16).padding(.top, 8).padding(.bottom, 16)
    }
}

internal struct Toast: View {
    @Environment(\.look) private var look
    internal let text: String

    internal var body: some View {
        HStack(spacing: 6) {
            Glyph(symbol: "globe", ascii: "[g]", size: 13).foregroundStyle(look.palette.lead)
            Text(text).ui(13, .medium, look)
        }
        .padding(.horizontal, 14).frame(height: 32).modifier(PillSurface(look: look))
    }
}

internal struct VoiceBubble: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 10) {
                Glyph(symbol: "play.fill", ascii: "[>]", size: 15)
                    .foregroundStyle(look.mono ? palette.link : (palette.dark ? Color.black : .white))
                    .frame(width: 40, height: 40)
                    .background(look.mono ? Color.clear : palette.link, in: .circle)
                VStack(alignment: .leading, spacing: 4) {
                    Wave(levels: Wave.note, tint: palette.link, played: 9)
                    Text("0:04 / 0:12").ui(12, .regular, look).foregroundStyle(palette.mute).monospacedDigit()
                }
            }
            Rectangle().fill(palette.line).frame(height: 0.5)
            Text("Rerun it fifty times on CI and post the flake rate, then tell me if it's still the sheet.")
                .ui(14, .regular, look).foregroundStyle(palette.foreground.opacity(0.85))
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(12).frame(width: 290, alignment: .leading)
        .background(look.mono ? Color.clear : palette.link.opacity(0.22), in: .rect(cornerRadius: 20))
        .overlay { if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) } }
        .frame(maxWidth: .infinity, alignment: .trailing)
    }
}

public struct ShellScreen: View {
    @Environment(\.look) private var look
    private let machine: Bool

    private var lines: [String] {
        if machine {
            return [
                "guy@mini ~ $ tail -f ~/Library/Logs/cooee/server.log", "09:41:02 pair: request from Guy's iPhone",
                "09:41:05 acp: session 7f3a started (coder)", "09:41:07 acp: tool rg settled web/e2e ok",
                "09:41:12 push: 1 sent", "^C", "guy@mini ~ $ ▍",
            ]
        }
        return [
            "coder@mini ~/src/cooee (issue-254) $ cargo test -p cooee-core", "   Compiling cooee-core v0.9.0",
            "    Finished test target(s) in 4.21s", "test result: ok. 214 passed; 0 failed", "",
            "coder@mini ~/src/cooee $ git status -s", " M web/e2e/look.spec.ts", "coder@mini ~/src/cooee $ vim ▍",
        ]
    }

    public var body: some View {
        let palette = look.palette
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                RoundButton(symbol: "chevron.left", ascii: "<", size: 40)
                VStack(alignment: .leading, spacing: 2) {
                    Text(look.mono ? "# terminal" : "Terminal").ui(16, .semibold, look)
                    Text(machine ? "mini · Machines" : "Coder · mini · ~/src/cooee").ui(12, .regular, look)
                        .foregroundStyle(palette.mute)
                }
                Spacer()
                RoundButton(symbol: "ellipsis", ascii: "…", size: 40)
            }
            .padding(.horizontal, 14).padding(.vertical, 8)
            Scroller(end: true) {
                VStack(alignment: .leading, spacing: 3) {
                    ForEach(Array(lines.enumerated()), id: \.offset) { _, line in
                        Text(line.isEmpty ? " " : line).font(.system(size: 13, design: .monospaced))
                            .foregroundStyle(line.contains("$") ? palette.green : palette.foreground.opacity(0.9))
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading).padding(12)
            }
            .frame(maxHeight: .infinity)
            .background(palette.dark ? Color.black.opacity(0.55) : Color.white.opacity(0.55))
            KeyboardView(kind: look.mono ? .cooee : .apple, keyRow: true, ctrl: machine)
        }
        .background(Backdrop()).foregroundStyle(palette.foreground)
    }

    public init(machine: Bool = false) {
        self.machine = machine
    }
}

public struct MacScreen: View {
    @Environment(\.look) private var look
    private let over: Over

    public var body: some View {
        MacChrome {
            WideView(personas: Fixtures.personas, chats: Fixtures.chats, selected: "t1", actions: HomeActions()) {
                ZStack {
                    VStack(spacing: 0) {
                        ChatHeader(persona: Story.coder, chips: [], back: false)
                        Scroller(end: true) { ThreadView(voice: true).frame(maxWidth: 760) }.frame(maxHeight: .infinity)
                        VStack(alignment: .leading, spacing: 8) {
                            if over == .slash { SlashList(typed: "/m").frame(width: 420) }
                            if over == .menu { MacMenu() }
                            ComposerBar(state: over == .slash ? .slash : .empty, wide: true)
                        }
                        .frame(maxWidth: 760).padding(.horizontal, 16).padding(.bottom, 20)
                    }
                    if case .attach(let tab) = over {
                        Dim()
                        AttachSheet(tab: tab).frame(width: 440).clipShape(.rect(cornerRadius: look.mono ? 0 : 22))
                    }
                }
            }
        }
        .background(Backdrop())
    }

    public init(over: Over = .slash) {
        self.over = over
    }
}

internal struct MacMenu: View {
    @Environment(\.look) private var look

    internal var body: some View {
        MenuList(agent: Array(Command.agent.prefix(3))).padding(14).frame(width: 420)
            .background(
                look.mono ? look.palette.background : look.palette.card, in: .rect(cornerRadius: look.mono ? 0 : 18)
            )
            .overlay {
                if look.mono { Rectangle().strokeBorder(look.palette.edge, lineWidth: 1) }
            }
            .shadow(color: .black.opacity(0.18), radius: 18, y: 6)
    }
}
