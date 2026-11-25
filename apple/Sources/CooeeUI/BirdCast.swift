import CooeeCore
import SwiftUI

extension Bird {
    private static let pink = Paint.mix(.red, .paper, 0.58)
    private static let grey = Paint.mix(.mute, .amber, 0.82)

    internal static func of(_ mascot: Mascot) -> Bird { cast[Mascot.allCases.firstIndex(of: mascot) ?? 0] }

    private static let cast = [kookaburra, galah, rosella, magpie, fairyWren, cockatoo, lorikeet, frogmouth]

    private static let kookaburra = Bird(
        paint: .amber,
        parts: Part.feet(.amber) + [
            .body(.amber), .oval(50, 71, 20, 14, .lead),
            .patch("M22 48 Q28 26 52 27 Q78 28 84 48 Q70 39 52 39 Q34 39 22 48Z", Paint.amber.shade(0.5)),
            .patch("M27 51 L2 57 Q14 58 27 56Z", Paint.amber.shade(0.3)),
            .patch("M27 56 Q14 58 4 58 Q14 62 27 63Z", .lead),
        ],
        eyes: Eyes(40, 55, 48, 13, 6.6))

    private static let galah = Bird(
        paint: pink,
        parts: Part.feet(pink) + [
            .body(pink),
            .patch("M30 40 Q40 24 58 26 Q74 28 80 40 Q66 34 52 34 Q40 34 30 40Z", Paint.paper.tint(0.75), edge: true),
            .patch("M50 27 Q52 16 62 18 Q58 22 58 28Z", Paint.paper.tint(0.75)),
            .beak(Paint.paper.tint(0.8), 23, 57, 10, 5),
        ],
        eyes: Eyes(40, 55, 47, 13, 6.6))

    private static let rosella = Bird(
        paint: .red,
        parts: [.bar(CGRect(x: 72, y: 56, width: 10, height: 34), 5, Paint.red.shade(0.6), turn: -28)]
            + Part.feet(.red) + [.body(.red), .oval(38, 64, 10, 8, .link), .beak(.lead, 23, 56, 10, 5)],
        eyes: Eyes(40, 55, 47, 13, 6.6))

    private static let magpie = Bird(
        paint: .paper,
        parts: Part.feet(.mute) + [
            .body(.paper), .patch("M21 60 Q18 30 50 28 Q80 28 84 52 Q70 46 60 56 Q46 66 21 60Z", .ink),
            .patch("M60 60 Q74 52 84 60 Q82 76 66 80 Q64 70 60 60Z", .ink), .beak(Paint.mute.tint(0.7), 23, 53, 12, 5),
        ],
        eyes: Eyes(40, 55, 44, 13, 6.6), eye: .paper)

    private static let fairyWren = Bird(
        paint: .link,
        parts: [.bar(CGRect(x: 68, y: 18, width: 9, height: 40), 4.5, Paint.link.shade(0.55), turn: 22)]
            + Part.feet(.link, 88, 44, 58) + [
                .body(.link, 52, 62, 28, 26), .oval(64, 52, 8, 7, Paint.link.tint(0.45)), .beak(.ink, 26, 58, 9, 4),
            ],
        eyes: Eyes(41, 54, 52, 11, 6.6))

    private static let cockatoo = Bird(
        paint: .lead,
        parts: [.stroke("M50 32 Q46 12 62 6 M56 32 Q58 14 74 12 M62 34 Q68 20 82 22", .amber, 6)] + Part.feet(.lead)
            + [.body(.lead), .patch("M27 50 Q14 52 16 62 Q20 58 27 62Z", Paint.mute.shade(0.45))],
        eyes: Eyes(40, 55, 48, 13, 6.6))

    private static let lorikeet = Bird(
        paint: .green,
        parts: Part.feet(.green) + [
            .body(.green), .patch("M22 54 Q20 28 52 28 Q80 28 84 46 Q66 38 52 44 Q36 50 22 54Z", .link),
            .oval(44, 72, 14, 10, .amber), .beak(.red, 24, 57, 10, 5),
        ],
        eyes: Eyes(41, 55, 44, 13, 6.6))

    private static let frogmouth = Bird(
        paint: grey,
        parts: Part.feet(grey, 90) + [
            .body(grey, 52, 57, 30, 34), .stroke("M33 36 Q38 30 46 34 M52 34 Q60 30 66 36", grey.shade(0.55), 3.5),
            .patch("M22 56 Q16 60 22 64 Q34 66 44 62 Q36 58 22 56Z", grey.shade(0.5)),
        ],
        eyes: Eyes(39, 57, 44, 6.5, 7.5))
}
