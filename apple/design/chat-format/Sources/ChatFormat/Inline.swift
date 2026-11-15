import SwiftUI

internal enum Inline {
    internal static func text(_ markdown: String, _ look: Look) -> AttributedString {
        let options = AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace)
        var out = (try? AttributedString(markdown: markdown, options: options)) ?? AttributedString(markdown)
        let palette = look.palette
        for run in out.runs {
            if run.inlinePresentationIntent?.contains(.code) == true {
                out[run.range].font = .system(size: look.bodySize - 1.5, design: .monospaced)
                if look.mono {
                    out[run.range].foregroundColor = palette.cyan
                } else {
                    out[run.range].foregroundColor = palette.lead
                    out[run.range].backgroundColor = palette.edge.opacity(palette.dark ? 0.45 : 0.4)
                }
            }
            if run.link != nil {
                out[run.range].foregroundColor = palette.link
                out[run.range].underlineStyle = look.style == .modern ? nil : .single
            }
        }
        return out
    }
}

internal enum Syntax {
    private static let keywords: Set<String> = [
        "func", "var", "let", "for", "in", "return", "if", "else", "guard", "public", "private", "internal", "static",
        "struct", "enum", "case", "import", "mutating", "try", "await", "async", "true", "false", "nil", "while",
        "switch", "fn", "pub", "mut", "use", "impl", "match", "self", "const", "def", "class",
    ]

    internal static func colour(_ code: String, lang: String, _ palette: Palette) -> AttributedString {
        var out = AttributedString()
        let hashComments = ["sh", "bash", "python", "toml", "yaml"].contains(lang)
        let chars = Array(code)
        var index = 0
        while index < chars.count {
            let (end, colour) = token(chars, at: index, hashComments: hashComments, palette)
            var piece = AttributedString(String(chars[index..<end]))
            piece.foregroundColor = colour
            out += piece
            index = end
        }
        return out
    }

    private static func token(_ chars: [Character], at start: Int, hashComments: Bool, _ palette: Palette)
        -> (Int, Color)
    {
        let char = chars[start]
        let next = start + 1 < chars.count ? chars[start + 1] : " "
        if (char == "/" && next == "/") || (hashComments && char == "#") {
            return (scan(chars, from: start) { $0 != "\n" }, palette.mute)
        }
        if char == "\"" {
            let end = scan(chars, from: start + 1) { $0 != "\"" && $0 != "\n" }
            return (min(end + 1, chars.count), palette.green)
        }
        if char.isNumber { return (scan(chars, from: start) { $0.isNumber || $0 == "." }, palette.amber) }
        if char.isLetter || char == "_" || char == "@" {
            let end = scan(chars, from: start + 1) { $0.isLetter || $0.isNumber || $0 == "_" }
            return (end, word(String(chars[start..<end]), call: end < chars.count && chars[end] == "(", palette))
        }
        return (start + 1, palette.foreground)
    }

    private static func word(_ word: String, call: Bool, _ palette: Palette) -> Color {
        if keywords.contains(word) { return palette.purple }
        if word.hasPrefix("@") { return palette.amber }
        if word.first?.isUppercase == true { return palette.cyan }
        return call ? palette.link : palette.foreground
    }

    private static func scan(_ chars: [Character], from start: Int, while keep: (Character) -> Bool) -> Int {
        var index = start
        while index < chars.count && keep(chars[index]) { index += 1 }
        return index
    }
}

internal enum ANSI {
    internal static func text(_ line: String, _ look: Look) -> AttributedString {
        let pieces = line.components(separatedBy: "\u{1B}[")
        var out = AttributedString(pieces[0])
        var colour = look.palette.foreground
        var bold = false
        for piece in pieces.dropFirst() {
            guard let end = piece.firstIndex(of: "m") else { continue }
            for code in piece[..<end].split(separator: ";").compactMap({ Int($0) }) {
                if code == 0 { colour = look.palette.foreground; bold = false }
                if code == 1 { bold = true }
                if let mapped = Self.colour(code, look.palette) { colour = mapped }
            }
            var run = AttributedString(String(piece[piece.index(after: end)...]))
            run.foregroundColor = colour
            run.font = look.code(bold ? .bold : .regular)
            out += run
        }
        return out
    }

    private static func colour(_ code: Int, _ palette: Palette) -> Color? {
        switch code {
        case 31, 91: palette.red
        case 32, 92: palette.green
        case 33, 93: palette.amber
        case 34, 94: palette.link
        case 35, 95: palette.purple
        case 36, 96: palette.cyan
        case 2, 90: palette.mute
        case 37, 39: palette.foreground
        default: nil
        }
    }
}
