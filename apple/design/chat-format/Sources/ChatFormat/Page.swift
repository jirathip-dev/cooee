import SwiftUI

public struct ChatPage: View {
    @Environment(\.look) private var look
    internal let entries: [Entry]

    public var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            ForEach(Array(entries.enumerated()), id: \.offset) { EntryView(entry: $0.element) }
        }
        .padding(.horizontal, 16).padding(.vertical, 14).frame(width: look.column + 32)
    }

    public init(entries: [Entry] = ChatFixture.entries) {
        self.entries = entries
    }
}

internal struct EntryView: View {
    @Environment(\.look) private var look
    internal let entry: Entry

    internal var body: some View {
        switch entry {
        case .you(let text): you(text)
        case .edit(let diff): DiffView(diff: diff)
        case .run(let run): RunView(run: run)
        case .agent(let parts):
            VStack(alignment: .leading, spacing: 12) {
                ForEach(Array(parts.enumerated()), id: \.offset) { PartView(part: $0.element) }
            }
        }
    }

    @ViewBuilder private func you(_ text: String) -> some View {
        let palette = look.palette
        if look.mono {
            HStack(alignment: .firstTextBaseline, spacing: 0) {
                Text("> you  ").foregroundStyle(palette.lead)
                Text(text)
            }
            .font(look.body())
        } else {
            HStack {
                Spacer(minLength: 60)
                Text(text).font(look.body()).padding(.horizontal, 14).padding(.vertical, 10)
                    .background(palette.lead.opacity(palette.dark ? 0.2 : 0.14), in: .rect(cornerRadius: 18))
            }
        }
    }
}

internal struct ChatHeader: View {
    @Environment(\.look) private var look
    internal let back: Bool

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 10) {
            if back { round(look.mono ? "<" : nil, symbol: "chevron.left") }
            Text(look.mono ? "o>_" : "CO").font(.system(size: 13, weight: .bold, design: .monospaced))
                .foregroundStyle(look.mono ? palette.cyan : palette.background)
                .frame(width: 38, height: 38)
                .background(look.mono ? Color.clear : palette.cyan, in: .circle)
                .overlay { if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) } }
            VStack(alignment: .leading, spacing: 3) {
                Text("Coder").font(.system(size: 16, weight: .semibold, design: look.mono ? .monospaced : .default))
                Text("Codex · gpt-5.5").font(look.small()).foregroundStyle(palette.mute)
            }
            Spacer()
            round(look.mono ? "…" : nil, symbol: "ellipsis")
        }
        .padding(.horizontal, 14).padding(.vertical, 8)
        .overlay(alignment: .bottom) { Rectangle().fill(palette.line).frame(height: 0.5) }
    }

    private func round(_ ascii: String?, symbol: String) -> some View {
        Group {
            if let ascii { Text("[\(ascii)]").font(look.body()) } else { Image(systemName: symbol) }
        }
        .font(.system(size: 15, weight: .semibold)).frame(width: 40, height: 40)
        .background(look.mono ? .clear : look.palette.edge.opacity(0.35), in: .circle)
    }
}

internal struct Backdrop: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        ZStack {
            palette.background
            if look.style == .modern {
                RadialGradient(
                    colors: [palette.lead.opacity(palette.dark ? 0.10 : 0.22), .clear], center: .topLeading,
                    startRadius: 10, endRadius: 420)
                RadialGradient(
                    colors: [palette.cyan.opacity(palette.dark ? 0.08 : 0.12), .clear], center: .bottomTrailing,
                    startRadius: 10, endRadius: 520)
            }
        }
    }
}

public struct PhoneFrame: View {
    private let look: Look

    public var body: some View {
        let foreground = look.palette.foreground
        VStack(spacing: 0) {
            ZStack {
                HStack {
                    Text("9:41").font(.system(size: 17, weight: .semibold)).padding(.leading, 34)
                    Spacer()
                    HStack(spacing: 6) {
                        Image(systemName: "cellularbars")
                        Image(systemName: "wifi")
                        Image(systemName: "battery.100")
                    }
                    .font(.system(size: 15, weight: .semibold)).padding(.trailing, 30)
                }
                Capsule().fill(.black).frame(width: 124, height: 36)
            }
            .frame(height: 54)
            ChatHeader(back: true)
            ChatPage()
            Capsule().fill(foreground.opacity(0.85)).frame(width: 140, height: 5).padding(.vertical, 8)
        }
        .frame(width: 402).foregroundStyle(foreground).background(Backdrop())
        .environment(\.look, look).environment(\.colorScheme, look.palette.dark ? .dark : .light)
    }

    public init(_ look: Look) {
        self.look = look
    }
}

public struct MacFrame: View {
    private let look: Look

    public var body: some View {
        let palette = look.palette
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                ForEach([0xff5f57, 0xfebc2e, 0x28c840] as [UInt32], id: \.self) {
                    Circle().fill(Color.hex($0)).frame(width: 12, height: 12)
                }
                ChatHeader(back: false).padding(.leading, 12)
            }
            .padding(.leading, 14)
            ChatPage().frame(maxWidth: .infinity)
        }
        .frame(width: 1100).foregroundStyle(palette.foreground).background(Backdrop())
        .clipShape(.rect(cornerRadius: 12))
        .environment(\.look, look).environment(\.colorScheme, palette.dark ? .dark : .light)
    }

    public init(_ look: Look) {
        self.look = look
    }
}
