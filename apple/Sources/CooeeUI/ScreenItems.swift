import CooeeCore
import SwiftUI

internal struct ScreenActs {
    internal var name = ""
    internal var open: (String) -> Void = { _ in }
    internal var take: () -> Void = {}
    internal var done: () -> Void = {}
}

private struct ScreenActsKey: EnvironmentKey {
    fileprivate static var defaultValue: ScreenActs { ScreenActs() }
}

extension EnvironmentValues {
    internal var screenActs: ScreenActs {
        get { self[ScreenActsKey.self] }
        set { self[ScreenActsKey.self] = newValue }
    }
}

internal struct ScreenItemView: View {
    @Environment(\.look) private var look
    @Environment(\.frames) private var frames
    @Environment(\.screenActs) private var acts
    internal let screen: ScreenItem

    internal var body: some View {
        switch screen {
        case .shot(let frame, let address, let time):
            shot(
                frame,
                footer:
                    "\(URL(string: address)?.host() ?? address) · \(time.formatted(date: .omitted, time: .shortened)) · a frame I asked for"
            )
        case .handoff(let what, let waiting, let note):
            HandoffCard(what: what, waiting: waiting, note: note)
        }
    }

    private func shot(_ frame: String, footer: String) -> some View {
        Button {
            acts.open(frame)
        } label: {
            VStack(alignment: .leading, spacing: 0) {
                if let image = FrameThumb.image(frames.images[frame]) {
                    Image(decorative: image, scale: 1).resizable().scaledToFit()
                } else {
                    look.palette.card.aspectRatio(1.6, contentMode: .fit).task { frames.load(frame) }
                }
                HStack {
                    Text(footer).lineLimit(1).foregroundStyle(look.palette.mute)
                    Spacer(minLength: 4)
                    Text("Open ›").foregroundStyle(look.palette.link)
                }
                .ui(12, .regular, look).padding(10)
            }
            .background(look.palette.card, in: .rect(cornerRadius: 12)).clipShape(.rect(cornerRadius: 12))
        }
        .buttonStyle(.plain).frame(maxWidth: 360, alignment: .leading).accessibilityLabel("Screen frame · \(footer)")
    }
}

internal struct HandoffCard: View {
    @Environment(\.look) private var look
    @Environment(\.screenActs) private var acts
    internal let what: String
    internal let waiting: Bool
    internal let note: String

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 8) {
            Text("✋ HANDOFF · \(acts.name)").ui(11, .bold, look).foregroundStyle(palette.amber)
            Text(what).ui(16, .semibold, look)
            if waiting {
                Text(
                    "Take control for 10 min and do it with your account. \(acts.name) never sees your password, and goes on once you hand back."
                )
                .ui(13, .regular, look).foregroundStyle(palette.mute).fixedSize(horizontal: false, vertical: true)
                HStack(spacing: 8) {
                    Button("Take control", action: acts.take).buttonStyle(.borderedProminent).tint(palette.amber)
                    Button("Done", action: acts.done).buttonStyle(.bordered)
                }
                .ui(14, .semibold, look)
            } else {
                Text(note.isEmpty ? "✓ You handed back" : "✓ You handed back · \(note)").ui(13, .regular, look)
                    .foregroundStyle(palette.green)
            }
        }
        .padding(12).frame(maxWidth: .infinity, alignment: .leading)
        .background(palette.card, in: .rect(cornerRadius: 14))
        .overlay(alignment: .leading) { Rectangle().fill(palette.amber).frame(width: 3) }
        .clipShape(.rect(cornerRadius: 14))
    }
}

internal struct WaitsPill: View {
    @Environment(\.look) private var look
    internal let name: String
    internal let since: Date

    internal var body: some View {
        TimelineView(.periodic(from: since, by: 1)) { context in
            let minutes = max(0, Int(context.date.timeIntervalSince(since))) / 60
            Label("\(name) waits for you · \(minutes) min", systemImage: "hand.raised.fill")
                .ui(13, .semibold, look).foregroundStyle(look.palette.red).lineLimit(1)
                .padding(.horizontal, 14).frame(maxWidth: .infinity, minHeight: 40, alignment: .leading)
                .background(look.palette.red.opacity(0.14), in: .capsule)
        }
    }
}
