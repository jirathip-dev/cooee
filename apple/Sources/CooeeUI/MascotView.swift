import CooeeCore
import SwiftUI

/// A persona's character as Modern draws it: its bird, moving in its state unless motion is off.
internal struct BirdView: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal let size: CGFloat

    internal var body: some View {
        let fills = look.palette.dark ? look.palette : .farmDark
        let sketch = Sketch(bird: Bird.of(persona.mascot), fills: fills, edge: !look.palette.dark)
        let failed = persona.state == .fail
        Clocked(seed: persona.seed) { time in
            Canvas { context, _ in
                context.scaleBy(x: size / 100, y: size / 100)
                sketch.draw(Stance(persona.state, at: time), persona.state, in: context)
            }
        }
        .frame(width: size, height: size).saturation(failed ? 0.25 : 1).brightness(failed ? -0.08 : 0)
        .accessibilityLabel("\(persona.mascot.name), \(look.word(persona.state))")
    }
}

/// A persona's character as Terminal draws it: its face in its colour, boxed unless `bare`.
internal struct FaceView: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal let size: CGFloat
    internal var bare = false

    internal var body: some View {
        let palette = look.palette
        let face = persona.mascot.face
        let tone = palette.color(face.paint)
        let ink = persona.state == .fail ? tone.mix(with: palette.mute, by: 0.5) : tone
        Clocked(seed: persona.seed) { time in
            Text(face.text(persona.state, at: time ?? 0)).font(
                .system(size: size * (bare ? 0.4 : 0.23), weight: .bold, design: .monospaced)
            )
            .foregroundStyle(ink).fixedSize()
        }
        .frame(width: bare ? nil : size, height: bare ? nil : size)
        .overlay { if !bare { Rectangle().strokeBorder(border(tone), lineWidth: 1) } }
        .accessibilityLabel("\(face.name), \(look.word(persona.state))")
    }

    private func border(_ tone: Color) -> Color {
        switch persona.state {
        case .needs: look.palette.amber
        case .fail: look.palette.red
        default: tone.opacity(0.7)
        }
    }
}

/// Seconds into its loop, out of step with other seeds, while motion is on; nil holds the key pose.
private struct Clocked<Content: View>: View {
    @Environment(\.look) private var look
    internal let seed: String
    @ViewBuilder internal let content: (Double?) -> Content

    internal var body: some View {
        if look.moving {
            TimelineView(.animation) { context in
                content(context.date.timeIntervalSinceReferenceDate + Mascot.phase(seed) * 4)
            }
        } else {
            content(nil)
        }
    }
}

private struct Sketch {
    internal let bird: Bird
    internal let fills: Palette
    internal let edge: Bool

    internal func draw(_ stance: Stance, _ state: LaneState, in context: GraphicsContext) {
        var figure = context
        figure.translateBy(x: 50 + stance.shift, y: 92 + stance.lift)
        figure.rotate(by: .degrees(stance.lean))
        figure.scaleBy(x: stance.wide, y: stance.tall)
        figure.translateBy(x: -50, y: -92)
        for item in bird.parts { part(figure, item) }
        eyes(figure, stance, state)
        switch state {
        case .working: dots(context, stance.dots)
        case .needs: bang(context, stance.pop)
        case .done, .merged: spark(context, stance)
        case .idle, .fail: break
        }
    }

    private func part(_ context: GraphicsContext, _ part: Part) {
        switch part {
        case .fill(let path, let paint, let rim):
            context.fill(path, with: .color(fills.color(paint)))
            if rim && edge { context.stroke(path, with: .color(fills.background.opacity(0.22)), lineWidth: 1.2) }
        case .line(let path, let paint, let width):
            context.stroke(path, with: .color(fills.color(paint)), style: Self.round(width))
        }
    }

    private func eyes(_ context: GraphicsContext, _ stance: Stance, _ state: LaneState) {
        let (left, right, y, high, wide) = bird.eyes
        let ink = GraphicsContext.Shading.color(fills.color(bird.eye))
        for x in [left, right] {
            switch state {
            case .done, .merged: context.stroke(Self.happy(x, y), with: ink, style: Self.round(3))
            case .fail: context.stroke(Self.cross(x, y), with: ink, style: Self.round(3))
            default:
                let tall = high * stance.lid
                let pill = CGRect(x: x - wide / 2 + stance.glance, y: y - tall / 2, width: wide, height: tall)
                context.fill(Path(roundedRect: pill, cornerRadius: min(wide, tall) / 2), with: ink)
            }
        }
    }

    private static func happy(_ x: CGFloat, _ y: CGFloat) -> Path {
        var path = Path()
        path.move(to: CGPoint(x: x - 4, y: y + 2))
        path.addQuadCurve(to: CGPoint(x: x + 4, y: y + 2), control: CGPoint(x: x, y: y - 5))
        return path
    }

    private static func cross(_ x: CGFloat, _ y: CGFloat, _ arm: CGFloat = 3.5) -> Path {
        var path = Path()
        path.addLines([CGPoint(x: x - arm, y: y - arm), CGPoint(x: x + arm, y: y + arm)])
        path.addLines([CGPoint(x: x + arm, y: y - arm), CGPoint(x: x - arm, y: y + arm)])
        return path
    }

    private func dots(_ context: GraphicsContext, _ shown: [Double]) {
        for (index, opacity) in shown.enumerated() {
            let rise = -3 * (opacity - 0.25) / 0.75
            let dot = CGRect(x: 73 + 8 * CGFloat(index), y: 15 + rise, width: 6, height: 6)
            context.fill(Path(ellipseIn: dot), with: .color(fills.foreground.opacity(opacity)))
        }
    }

    private func bang(_ context: GraphicsContext, _ pop: Double) {
        var mark = context
        mark.translateBy(x: 87, y: 16)
        mark.scaleBy(x: pop, y: pop)
        mark.translateBy(x: -87, y: -16)
        let ink = GraphicsContext.Shading.color(fills.background)
        mark.fill(
            Path(roundedRect: CGRect(x: 78, y: 4, width: 18, height: 24), cornerRadius: 9), with: .color(fills.amber))
        mark.fill(Path(roundedRect: CGRect(x: 85.5, y: 8, width: 3, height: 10), cornerRadius: 1.5), with: ink)
        mark.fill(Path(ellipseIn: CGRect(x: 85.2, y: 20.2, width: 3.6, height: 3.6)), with: ink)
    }

    private func spark(_ context: GraphicsContext, _ stance: Stance) {
        var mark = context
        mark.opacity = stance.spark.shown
        mark.translateBy(x: 84, y: 18)
        mark.rotate(by: .degrees(stance.spark.turn))
        mark.scaleBy(x: stance.spark.size, y: stance.spark.size)
        mark.stroke(
            Self.cross(0, 0, 6 / 2.squareRoot()).applying(CGAffineTransform(rotationAngle: .pi / 4)),
            with: .color(fills.green), style: Self.round(3))
    }

    private static func round(_ width: CGFloat) -> StrokeStyle {
        StrokeStyle(lineWidth: width, lineCap: .round, lineJoin: .round)
    }
}

extension Palette {
    /// A character's paint in this scheme: `fg` and `paper` are its text colour, `ink` its background.
    internal func color(_ paint: Paint) -> Color {
        if case .mix(let first, let second, let share) = paint {
            return color(first).mix(with: color(second), by: 1 - share, in: .device)
        }
        return Self.tokens[paint]?(self) ?? foreground
    }

    private static let tokens: [Paint: @Sendable (Palette) -> Color] = [
        .lead: \.lead, .link: \.link, .green: \.green, .amber: \.amber, .red: \.red, .purple: \.purple,
        .cyan: \.cyan, .fg: \.foreground, .paper: \.foreground, .mute: \.mute, .ink: \.background,
    ]
}
