import CooeeCore
import SwiftUI

internal struct CallActs {
    internal var muted = false
    internal var speaker = false
    internal var mute: () -> Void = {}
    internal var loud: () -> Void = {}
    internal var thread: () -> Void = {}
    internal var end: () -> Void = {}
}

internal struct CallScreen: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal let title: String
    internal let seconds: Int
    internal let lines: [Call.Line]
    internal let speaking: String?
    internal let tool: String?
    internal var acts = CallActs()

    internal var body: some View {
        let palette = look.palette
        ZStack {
            Backdrop()
            VStack(spacing: 14) {
                Text(look.type == .mono ? "> call \(persona.name.lowercased())" : "Call with \(persona.name)")
                    .ui(15, .semibold, look).foregroundStyle(palette.mute)
                Text([title, CallBlock.clock(seconds)].filter { !$0.isEmpty }.joined(separator: " · "))
                    .ui(13, .regular, look).foregroundStyle(palette.mute)
                rings.padding(.vertical, 8)
                transcript
                Spacer(minLength: 0)
                controls
            }
            .padding(.horizontal, 18).padding(.top, 14).padding(.bottom, 34)
        }
        .foregroundStyle(palette.foreground)
    }

    private var rings: some View {
        let tint = look.palette.tint(persona.look.tint)
        return ZStack {
            if look.type != .mono {
                ForEach([1.9, 1.55, 1.25], id: \.self) { scale in
                    Circle().stroke(tint.opacity(0.5 / scale), lineWidth: 2).frame(width: 96 * scale)
                }
            }
            FigureView(persona: persona, size: 96)
        }
        .frame(height: 190)
    }

    private var transcript: some View {
        let palette = look.palette
        let said = speaking != nil && lines.last?.you == false ? lines.dropLast() : lines[...]
        return VStack(alignment: .leading, spacing: 8) {
            ForEach(Array(said.suffix(3).enumerated()), id: \.offset) { _, line in
                Text(line.text).ui(14, .regular, look).foregroundStyle(line.you ? palette.mute : palette.foreground)
                    .frame(maxWidth: .infinity, alignment: line.you ? .trailing : .leading)
            }
            if let speaking {
                Text("\(speaking) ▍").ui(20, .semibold, look).fixedSize(horizontal: false, vertical: true)
            }
            if let tool {
                HStack(spacing: 6) {
                    Mark(text: LaneState.working.figure(frame: 0), size: 12) {
                        Circle().fill(palette.cyan).frame(width: 8, height: 8)
                    }
                    Text(tool).font(
                        .system(size: 12, design: look.stage == nil || look.type == .mono ? .monospaced : .default))
                }
                .foregroundStyle(palette.mute)
            }
        }
    }

    private var controls: some View {
        HStack(spacing: 22) {
            control("mic.slash.fill", "Mute", lit: acts.muted, run: acts.mute)
            control("speaker.wave.2.fill", "Speaker", lit: acts.speaker, run: acts.loud)
            control("text.bubble.fill", "Thread", run: acts.thread)
            control("phone.down.fill", "End", run: acts.end)
        }
    }

    private func control(_ symbol: String, _ title: String, lit: Bool = false, run: @escaping () -> Void) -> some View {
        let palette = look.palette
        return Button(action: run) {
            VStack(spacing: 6) {
                if title == "End" {
                    Image(systemName: symbol).font(.system(size: 24, weight: .semibold)).foregroundStyle(.white)
                        .frame(width: 64, height: 64)
                        .background(palette.red, in: look.square ? AnyShape(Rectangle()) : AnyShape(Circle()))
                } else {
                    RoundButton(symbol: symbol, ascii: String(title.prefix(1)).lowercased(), size: 64)
                }
                Text(title).ui(12, .medium, look).foregroundStyle(lit ? palette.link : palette.mute)
            }
        }
        .buttonStyle(.plain).accessibilityLabel(title).accessibilityAddTraits(lit ? .isSelected : [])
    }
}
