import CooeeCore
import SwiftUI

internal struct WorkingPill: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal var since: Date?
    internal var step: String?
    internal var stop: () -> Void = {}

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 8) {
            Acting(motion: .walk) { pose in StateSprite(persona: persona.with(state: .working), frame: pose.frame) }
            if let since {
                TimelineView(.periodic(from: since, by: 1)) { context in
                    label(Int(context.date.timeIntervalSince(since)))
                }
            } else {
                label(42)
            }
            Spacer()
            Button(action: stop) {
                HStack(spacing: 4) {
                    Image(systemName: "stop.fill").font(.system(size: 10))
                    Text("Stop")
                }
                .ui(13, .semibold, look).foregroundStyle(palette.red)
                .padding(.horizontal, 10).frame(height: 28).background(palette.red.opacity(0.14), in: .capsule)
            }
            .buttonStyle(.plain).accessibilityLabel("Stop")
        }
        .padding(.leading, 14).padding(.trailing, 6).frame(height: 40)
        .surface(look, radius: 20)
    }

    private func label(_ seconds: Int) -> some View {
        let time = "\(max(0, seconds) / 60):\(String(format: "%02d", max(0, seconds) % 60))"
        let what = step ?? (look.mono ? "working" : "Working")
        return Text(look.mono ? "\(what) \(time)" : "\(what) · \(time)").ui(13, .medium, look).lineLimit(1)
            .truncationMode(.middle)
    }
}
