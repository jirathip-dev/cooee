import CooeeCore
import SwiftUI

internal enum Syntax {
    private static let keywords: Set<String> = [
        "func", "var", "let", "for", "in", "return", "if", "else", "guard", "public", "private", "internal", "static",
        "struct", "enum", "case", "import", "mutating", "try", "await", "async", "true", "false", "nil", "while",
        "switch", "fn", "pub", "mut", "use", "impl", "match", "self", "const", "def", "class", "function", "export",
    ]
    private static let hashComments: Set<String> = ["sh", "bash", "zsh", "shell", "python", "py", "toml", "yaml", "rb"]

    internal static func colour(_ code: String, lang: String, _ palette: Palette) -> AttributedString {
        var out = AttributedString()
        let chars = Array(code)
        var index = 0
        while index < chars.count {
            let (end, colour) = token(chars, at: index, hashComments: hashComments.contains(lang), palette)
            var piece = AttributedString(String(chars[index..<end]))
            piece.foregroundColor = colour
            out += piece
            index = end
        }
        return out
    }

    internal static func ansi(_ line: String, _ palette: Palette) -> AttributedString {
        var out = AttributedString()
        for run in ANSI.runs(line) {
            var piece = AttributedString(run.text)
            piece.foregroundColor = run.colour.flatMap { tones(palette)[$0] } ?? palette.foreground
            piece.font = .system(size: 13, weight: run.bold ? .bold : .regular, design: .monospaced)
            out += piece
        }
        return out
    }

    private static func tones(_ palette: Palette) -> [Int: Color] {
        [
            31: palette.red, 32: palette.green, 33: palette.amber, 34: palette.link, 35: palette.purple,
            36: palette.cyan, 90: palette.mute,
        ]
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
