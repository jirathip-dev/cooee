import CooeeCore
import SwiftUI

/// One flat part of a figure, in its 100 × 100 box (apple/design/agent-birds/cast.js).
internal enum Part: Sendable {
    case fill(Path, Paint, edge: Bool = false)
    case line(Path, Paint, width: CGFloat)

    internal static func oval(_ x: CGFloat, _ y: CGFloat, _ rx: CGFloat, _ ry: CGFloat, _ paint: Paint) -> Part {
        .fill(Path(ellipseIn: CGRect(x: x - rx, y: y - ry, width: rx * 2, height: ry * 2)), paint)
    }

    internal static func bar(_ rect: CGRect, _ radius: CGFloat, _ paint: Paint, turn: Double = 0) -> Part {
        let spin = CGAffineTransform(translationX: rect.midX, y: rect.midY).rotated(by: turn * .pi / 180)
            .translatedBy(x: -rect.midX, y: -rect.midY)
        return .fill(Path(roundedRect: rect, cornerRadius: radius).applying(spin), paint)
    }

    internal static func body(
        _ paint: Paint, _ x: CGFloat = 52, _ y: CGFloat = 58, _ rx: CGFloat = 32, _ ry: CGFloat = 30
    )
        -> Part
    {
        .fill(Path(ellipseIn: CGRect(x: x - rx, y: y - ry, width: rx * 2, height: ry * 2)), paint, edge: true)
    }

    internal static func shape(_ data: String, _ paint: Paint, edge: Bool = false) -> Part {
        .fill(svg(data), paint, edge: edge)
    }

    internal static func stroke(_ data: String, _ paint: Paint, _ width: CGFloat) -> Part {
        .line(svg(data), paint, width: width)
    }

    internal static func feet(_ paint: Paint, _ y: CGFloat = 88, _ left: CGFloat = 42, _ right: CGFloat = 58) -> [Part]
    {
        [left, right].map { bar(CGRect(x: $0 - 4, y: y - 2, width: 8, height: 5), 2.5, paint.shade(0.55)) }
    }

    internal static func beak(_ paint: Paint, _ x: CGFloat, _ y: CGFloat, _ long: CGFloat, _ high: CGFloat) -> Part {
        var path = Path()
        path.move(to: CGPoint(x: x, y: y - high))
        path.addQuadCurve(to: CGPoint(x: x - long, y: y), control: CGPoint(x: x - long * 0.3, y: y - high * 0.6))
        path.addQuadCurve(to: CGPoint(x: x, y: y + high), control: CGPoint(x: x - long * 0.3, y: y + high * 0.6))
        path.closeSubpath()
        return .fill(path, paint)
    }

    /// An SVG path of absolute `M`, `L`, `Q` and `Z` commands, the only ones the cast uses.
    internal static func svg(_ data: String) -> Path {
        let words = data.map { $0.isLetter ? " \($0) " : String($0) }.joined().split(separator: " ")
        let numbers = words.map { Double($0) ?? 0 }
        let point = { (at: Int) in CGPoint(x: numbers[at], y: numbers[at + 1]) }
        var path = Path()
        var index = 0
        while index < words.count {
            switch words[index] {
            case "M": path.move(to: point(index + 1))
            case "L": path.addLine(to: point(index + 1))
            case "Q": path.addQuadCurve(to: point(index + 3), control: point(index + 1))
            default: path.closeSubpath()
            }
            index += ["M": 3, "L": 3, "Q": 5][words[index]] ?? 1
        }
        return path
    }
}

/// A Modern bird: its parts, then its two pill eyes.
internal struct Bird: Sendable {
    internal let paint: Paint
    internal let parts: [Part]
    internal let eyes: (left: CGFloat, right: CGFloat, y: CGFloat, high: CGFloat, wide: CGFloat)
    internal var eye = Paint.ink
}
