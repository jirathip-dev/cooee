import CooeeCore
import SwiftUI

internal enum Part: Sendable {
    case fill(Path, Paint, edge: Bool = false)
    case line(Path, Paint, width: CGFloat)

    internal static func oval(
        _ midX: CGFloat, _ midY: CGFloat, _ halfWide: CGFloat, _ halfHigh: CGFloat, _ paint: Paint
    ) -> Part {
        .fill(
            Path(ellipseIn: CGRect(x: midX - halfWide, y: midY - halfHigh, width: halfWide * 2, height: halfHigh * 2)),
            paint)
    }

    internal static func bar(_ rect: CGRect, _ radius: CGFloat, _ paint: Paint, turn: Double = 0) -> Part {
        let spin = CGAffineTransform(translationX: rect.midX, y: rect.midY).rotated(by: turn * .pi / 180)
            .translatedBy(x: -rect.midX, y: -rect.midY)
        return .fill(Path(roundedRect: rect, cornerRadius: radius).applying(spin), paint)
    }

    internal static func body(
        _ paint: Paint, _ midX: CGFloat = 52, _ midY: CGFloat = 58, _ halfWide: CGFloat = 32, _ halfHigh: CGFloat = 30
    ) -> Part {
        .fill(
            Path(ellipseIn: CGRect(x: midX - halfWide, y: midY - halfHigh, width: halfWide * 2, height: halfHigh * 2)),
            paint, edge: true)
    }

    internal static func patch(_ data: String, _ paint: Paint, edge: Bool = false) -> Part {
        .fill(svg(data), paint, edge: edge)
    }

    internal static func stroke(_ data: String, _ paint: Paint, _ width: CGFloat) -> Part {
        .line(svg(data), paint, width: width)
    }

    internal static func feet(_ paint: Paint, _ ground: CGFloat = 88, _ left: CGFloat = 42, _ right: CGFloat = 58)
        -> [Part]
    {
        [left, right].map { bar(CGRect(x: $0 - 4, y: ground - 2, width: 8, height: 5), 2.5, paint.shade(0.55)) }
    }

    internal static func beak(_ paint: Paint, _ rootX: CGFloat, _ rootY: CGFloat, _ long: CGFloat, _ high: CGFloat)
        -> Part
    {
        var path = Path()
        path.move(to: CGPoint(x: rootX, y: rootY - high))
        path.addQuadCurve(
            to: CGPoint(x: rootX - long, y: rootY), control: CGPoint(x: rootX - long * 0.3, y: rootY - high * 0.6))
        path.addQuadCurve(
            to: CGPoint(x: rootX, y: rootY + high), control: CGPoint(x: rootX - long * 0.3, y: rootY + high * 0.6))
        path.closeSubpath()
        return .fill(path, paint)
    }

    internal static func svg(_ data: String) -> Path {
        let words = data.map { $0.isLetter ? " \($0) " : String($0) }.joined().split(separator: " ")
        let numbers = words.map { Double($0) ?? 0 }
        let point = { (slot: Int) in CGPoint(x: numbers[slot], y: numbers[slot + 1]) }
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

internal struct Bird: Sendable {
    internal let paint: Paint
    internal let parts: [Part]
    internal let eyes: Eyes
    internal var eye = Paint.ink
}

internal struct Eyes: Sendable {
    internal let left: CGFloat
    internal let right: CGFloat
    internal let level: CGFloat
    internal let high: CGFloat
    internal let wide: CGFloat

    internal init(_ left: CGFloat, _ right: CGFloat, _ level: CGFloat, _ high: CGFloat, _ wide: CGFloat) {
        (self.left, self.right, self.level, self.high, self.wide) = (left, right, level, high, wide)
    }
}
