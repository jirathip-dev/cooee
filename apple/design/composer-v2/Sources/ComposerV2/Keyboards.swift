import CooeeCore
@testable import CooeeUI
import SwiftUI

internal enum Cap: Sendable {
    case letter, special, accent
}

internal struct Key: Sendable {
    internal let label: String
    internal var symbol: String?
    internal var width: CGFloat = 1
    internal var cap: Cap = .letter

    internal static func row(_ letters: String) -> [Key] {
        letters.unicodeScalars.filter { $0 != " " }.map { Key(label: String($0)) }
    }
}

internal struct KeyboardView: View {
    @Environment(\.look) private var look
    internal let kind: Keyboard
    internal var keyRow = false
    internal var ctrl = false

    internal var body: some View {
        VStack(spacing: 0) {
            if keyRow { TerminalKeyRow(ctrl: ctrl, apple: kind != .cooee) }
            if kind == .cooee { CooeeKeyboard(shell: keyRow) } else { AppleKeyboard(thai: kind == .thai, shell: keyRow) }
        }
    }
}

internal struct AppleKeyboard: View {
    @Environment(\.look) private var look
    internal let thai: Bool
    internal var shell = false

    private var rows: [[Key]] {
        if thai {
            return [
                Key.row("ไ ำ พ ะ ั ี ร น ย บ ล"), Key.row("ฟ ห ก ด เ ้ ่ า ส ว ง"),
                [Key(label: "", symbol: "shift", width: 1.3, cap: .special)] + Key.row("ผ ป แ อ ิ ื ท ม ใ")
                    + [Key(label: "", symbol: "delete.left", width: 1.3, cap: .special)],
                bottom("เว้นวรรค", "ส่ง"),
            ]
        }
        return [
            Key.row("q w e r t y u i o p"), Key.row("a s d f g h j k l"),
            [Key(label: "", symbol: "shift", width: 1.3, cap: .special)] + Key.row("z x c v b n m")
                + [Key(label: "", symbol: "delete.left", width: 1.3, cap: .special)],
            bottom("space", shell ? "return" : "send"),
        ]
    }

    private func bottom(_ space: String, _ send: String) -> [Key] {
        [
            Key(label: "123", width: 1.3, cap: .special), Key(label: "", symbol: "face.smiling", width: 1.3, cap: .special),
            Key(label: space, width: 5), Key(label: send, width: 2.4, cap: .accent),
        ]
    }

    internal var body: some View {
        let dark = look.palette.dark
        VStack(spacing: 11) {
            ForEach(Array(rows.enumerated()), id: \.offset) { _, row in KeyLine(keys: row, mono: false) }
            HStack {
                Image(systemName: "globe")
                Spacer()
                Image(systemName: "mic")
            }
            .font(.system(size: 22)).foregroundStyle(dark ? Color.white : .black)
            .padding(.horizontal, 28).padding(.top, 6).padding(.bottom, 26)
        }
        .padding(.horizontal, 4).padding(.top, 10)
        .background(
            UnevenRoundedRectangle(topLeadingRadius: 26, topTrailingRadius: 26)
                .fill(dark ? Color.hex(0x1f2020) : Color.hex(0xd3d6dc))
        )
    }
}

internal struct CooeeKeyboard: View {
    @Environment(\.look) private var look

    internal var shell = false

    private var rows: [[Key]] {
        [
        Key.row("1 2 3 4 5 6 7 8 9 0"), Key.row("q w e r t y u i o p"), Key.row("a s d f g h j k l"),
        [Key(label: "⇧", width: 1.3, cap: .special)] + Key.row("z x c v b n m")
            + [Key(label: "⌫", width: 1.3, cap: .special)],
        [
            Key(label: "", symbol: "globe", width: 1.3, cap: .special), Key(label: "#+=", width: 1.3, cap: .special),
            Key(label: "space", width: 4.4), Key(label: "/", cap: .special), Key(label: shell ? "ret ↵" : "send ↵", width: 2, cap: .accent),
            ],
        ]
    }

    internal var body: some View {
        VStack(spacing: 9) {
            ForEach(Array(rows.enumerated()), id: \.offset) { _, row in KeyLine(keys: row, mono: true) }
        }
        .padding(.horizontal, 4).padding(.top, 9).padding(.bottom, 34)
        .background(look.palette.card.opacity(0.9))
        .overlay(alignment: .top) {
            Rectangle().fill(look.palette.edge).frame(height: 1)
        }
    }
}

internal struct KeyLine: View {
    @Environment(\.look) private var look
    internal let keys: [Key]
    internal let mono: Bool

    internal var body: some View {
        GeometryReader { proxy in
            let units = keys.reduce(0) { $0 + $1.width }
            let unit = (proxy.size.width - CGFloat(keys.count - 1) * 6) / max(units, 10)
            HStack(spacing: 6) {
                ForEach(Array(keys.enumerated()), id: \.offset) { _, key in
                    Keycap(key: key, mono: mono).frame(width: unit * key.width)
                }
            }
            .frame(maxWidth: .infinity)
        }
        .frame(height: mono ? 40 : 44)
    }
}

internal struct Keycap: View {
    @Environment(\.look) private var look
    internal let key: Key
    internal let mono: Bool

    internal var body: some View {
        let palette = look.palette
        Group {
            if let symbol = key.symbol {
                Image(systemName: symbol).font(.system(size: 18, weight: .regular))
            } else {
                Text(key.label).font(
                    .system(size: key.label.count > 1 ? 15 : (mono ? 18 : 23), design: mono ? .monospaced : .default)
                )
            }
        }
        .foregroundStyle(ink(palette)).frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(fill(palette), in: mono ? AnyShape(Rectangle()) : AnyShape(.rect(cornerRadius: 8)))
        .overlay {
            if mono {
                Rectangle().strokeBorder(key.symbol == "globe" ? palette.lead : palette.edge, lineWidth: 1)
            }
        }
        .shadow(color: .black.opacity(mono ? 0 : 0.25), radius: 0, y: 1)
    }

    private func ink(_ palette: Palette) -> Color {
        if mono { return key.cap == .accent ? palette.link : (key.symbol == "globe" ? palette.lead : palette.foreground) }
        if key.cap == .accent { return .white }
        return palette.dark ? .white : .black
    }

    private func fill(_ palette: Palette) -> Color {
        if mono { return key.cap == .letter ? palette.background : palette.card }
        switch key.cap {
        case .accent: return Color.hex(0x0a84ff)
        case .special: return palette.dark ? Color.hex(0x3f3f42) : Color.hex(0xacb1bb)
        case .letter: return palette.dark ? Color.hex(0x6b6b6e) : .white
        }
    }
}

internal struct TerminalKeyRow: View {
    @Environment(\.look) private var look
    internal let ctrl: Bool
    internal let apple: Bool

    private static let keys = ["esc", "tab", "ctrl", "←", "↑", "↓", "→", "/", "|", "~", "-"]

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 4) {
            ForEach(Self.keys, id: \.self) { key in
                let lit = ctrl && key == "ctrl"
                Text(key).font(.system(size: key.count > 1 ? 13 : 16, weight: .medium, design: .monospaced))
                    .foregroundStyle(lit ? (palette.dark ? Color.black : .white) : palette.foreground)
                    .frame(maxWidth: .infinity).frame(height: 34)
                    .background(lit ? palette.link : palette.card, in: look.mono ? AnyShape(Rectangle()) : AnyShape(.rect(cornerRadius: 7)))
                    .overlay { if look.mono { Rectangle().strokeBorder(palette.edge, lineWidth: 1) } }
                    .frame(width: key.count > 1 ? 40 : 26)
            }
        }
        .padding(.horizontal, 6).padding(.vertical, 6)
        .background(look.mono ? palette.background : palette.background.opacity(0.92))
        .overlay(alignment: .top) { Rectangle().fill(palette.line).frame(height: 0.5) }
    }
}
