import CooeeCore
@testable import CooeeUI
import SwiftUI

internal struct Spoken: Identifiable, Sendable {
    internal let id: Int
    internal let you: Bool
    internal let text: String

    internal static let lines = [
        Spoken(id: 1, you: true, text: "What did the Q3 10-Q say about Birch?"),
        Spoken(id: 2, you: false, text: "Birch is 14% of revenue, down from 19% in Q2."),
        Spoken(id: 3, you: true, text: "Why the drop?"),
    ]
}

public struct CallScreen: View {
    @Environment(\.look) private var look
    private let researcher = Persona.named("res", .working)

    public var body: some View {
        let palette = look.palette
        ZStack {
            Backdrop()
            VStack(spacing: 14) {
                Text(look.mono ? "> call researcher" : "Call with Researcher").ui(15, .semibold, look)
                    .foregroundStyle(palette.mute)
                Text("Kestrel filings · 3:12").ui(13, .regular, look).foregroundStyle(palette.mute)
                rings.padding(.vertical, 8)
                transcript
                Spacer(minLength: 0)
                Text("On-device voice, no account needed · every line is kept in the thread")
                    .ui(12, .regular, look).foregroundStyle(palette.mute).multilineTextAlignment(.center)
                controls
            }
            .padding(.horizontal, 18).padding(.top, 14).padding(.bottom, 34)
        }
        .foregroundStyle(palette.foreground)
    }

    private var rings: some View {
        let tint = look.palette.tint(researcher.look.tint)
        return ZStack {
            if !look.mono {
                ForEach([1.9, 1.55, 1.25], id: \.self) { scale in
                    Circle().stroke(tint.opacity(0.5 / scale), lineWidth: 2).frame(width: 96 * scale)
                }
            }
            Avatar(persona: researcher, size: 96)
        }
        .frame(height: 190)
    }

    private var transcript: some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 8) {
            ForEach(Spoken.lines) { line in
                Text(line.text).ui(14, .regular, look).foregroundStyle(line.you ? palette.mute : palette.foreground)
                    .frame(maxWidth: .infinity, alignment: line.you ? .trailing : .leading)
            }
            Text("The Birch contract renews in January, so it's deferred. ▍").ui(20, .semibold, look)
                .fixedSize(horizontal: false, vertical: true)
            HStack(spacing: 6) {
                StateMark(state: .working)
                Text("fetch sec.gov … notes, p. 14").ui(12, .regular, look.with(mono: true))
            }
            .foregroundStyle(palette.mute)
        }
    }

    private var controls: some View {
        HStack(spacing: 22) {
            control("mic.slash.fill", "Mute")
            control("speaker.wave.2.fill", "Speaker")
            control("text.bubble.fill", "Thread")
            control("phone.down.fill", "End", end: true)
        }
    }

    private func control(_ symbol: String, _ title: String, end: Bool = false) -> some View {
        let palette = look.palette
        return VStack(spacing: 6) {
            Group {
                if end {
                    Image(systemName: symbol).font(.system(size: 24, weight: .semibold)).foregroundStyle(.white)
                        .frame(width: 64, height: 64)
                        .background(palette.red, in: look.mono ? AnyShape(Rectangle()) : AnyShape(Circle()))
                } else {
                    RoundButton(symbol: symbol, ascii: String(title.prefix(1)).lowercased(), size: 64)
                }
            }
            Text(title).ui(12, .medium, look).foregroundStyle(palette.mute)
        }
    }

    public init() {}
}
