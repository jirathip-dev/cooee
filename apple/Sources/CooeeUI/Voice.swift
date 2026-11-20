import CooeeCore
import CooeeFeatures
import SwiftUI

/// The composer's voice: the dictation shown, and the mic's tap, hold and picks; the header's call button.
public struct VoiceActs {
    public var dictation: Dictation?
    public var mic: () -> Void = {}
    public var hold: (Bool) -> Void = { _ in }
    public var pick: (String) -> Void = { _ in }
    public var call: (() -> Void)?

    public var listening: Bool { dictation?.listening == true }

    public init(dictation: Dictation? = nil) {
        self.dictation = dictation
    }
}

internal struct MicButton: View {
    @Environment(\.look) private var look
    private let holding = State(initialValue: false)
    internal let voice: VoiceActs

    internal var body: some View {
        let palette = look.palette
        Group {
            if voice.listening {
                Image(systemName: "stop.fill").font(.system(size: 11)).foregroundStyle(.white)
                    .frame(width: 26, height: 26).background(
                        palette.red, in: look.square ? AnyShape(Rectangle()) : AnyShape(Circle()))
            } else {
                Image(systemName: "mic").foregroundStyle(holding.wrappedValue ? palette.red : palette.mute)
                    .frame(width: voice.dictation == nil ? nil : 26, height: voice.dictation == nil ? nil : 26)
            }
        }
        .contentShape(.rect).onTapGesture(perform: voice.mic)
        .onLongPressGesture(minimumDuration: 0.35) {
            holding.wrappedValue = true
            voice.hold(true)
        } onPressingChanged: { pressing in
            guard !pressing, holding.wrappedValue else { return }
            holding.wrappedValue = false
            voice.hold(false)
        }
        .accessibilityLabel(voice.listening ? "Stop dictation" : "Dictate")
        .accessibilityAction(named: "Record a voice note") { voice.hold(true) }
    }
}

internal struct DictationWords: View {
    @Environment(\.look) private var look
    internal let dictation: Dictation

    internal var body: some View {
        let pending = dictation.heard.pending
        let said = [dictation.before, dictation.heard.settled].filter { !$0.isEmpty }.joined(separator: " ")
        VStack(alignment: .leading, spacing: 8) {
            Wave()
            Text("\(said) \(Text(pending.isEmpty ? "▍" : "\(pending) ▍").foregroundStyle(look.palette.mute))")
                .ui(16, .regular, look).fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    internal static func underlined(_ text: AttributedString, _ dictation: Dictation?, _ tint: Color)
        -> AttributedString
    {
        var text = text
        for unsure in dictation?.heard.unsure ?? [] {
            guard let range = text.range(of: unsure.word) else { continue }
            text[range].swiftUI.underlineStyle = Text.LineStyle(pattern: .solid, color: tint)
        }
        return text
    }
}

internal struct Alternatives: View {
    @Environment(\.look) private var look
    internal let unsure: Unsure
    internal let pick: (String) -> Void

    internal var body: some View {
        HStack(spacing: 6) {
            Text("\(unsure.word) →").ui(12, .regular, look).foregroundStyle(look.palette.mute)
            ForEach(unsure.alternatives, id: \.self) { word in
                Button {
                    pick(word)
                } label: {
                    Text(word).ui(13, .semibold, look).foregroundStyle(look.palette.link)
                        .padding(.horizontal, 12).frame(height: 30).chip(look)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.leading, 56)
    }
}

internal struct Wave: View {
    @Environment(\.look) private var look
    private static let levels: [CGFloat] = [4, 9, 15, 22, 12, 6, 18, 26, 14, 8, 20, 11, 5, 16, 24, 10, 6, 13, 19, 7]

    internal var body: some View {
        HStack(alignment: .center, spacing: 3) {
            ForEach(Array(Self.levels.enumerated()), id: \.offset) { _, level in
                Capsule().fill(look.palette.red.opacity(0.85)).frame(width: 3, height: level)
            }
        }
        .frame(height: 26)
    }
}

internal struct CallBlock: View {
    @Environment(\.look) private var look
    @Environment(\.asker) private var asker
    private let all = State(initialValue: false)
    internal let call: Call

    internal var body: some View {
        let palette = look.palette
        let shown = all.wrappedValue ? call.lines : Array(call.lines.prefix(3))
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 6) {
                Image(systemName: "phone.fill").foregroundStyle(palette.green)
                Text(
                    look.label(.chip, "Call")
                        + " · \(Self.clock(call.seconds)) · \(call.lines.count) lines · \(call.tools) tools"
                )
                .ui(13, .semibold, look)
            }
            ForEach(Array(shown.enumerated()), id: \.offset) { _, line in
                HStack(alignment: .top, spacing: 6) {
                    Text(line.you ? "You" : asker?.name ?? "").ui(13, .semibold, look)
                        .foregroundStyle(line.you ? palette.mute : palette.purple).frame(width: 78, alignment: .leading)
                    Text(line.text).ui(13, .regular, look).fixedSize(horizontal: false, vertical: true)
                }
            }
            if call.lines.count > shown.count {
                Button("Show all \(call.lines.count) lines") { all.wrappedValue = true }.buttonStyle(.plain)
                    .ui(13, .semibold, look).foregroundStyle(palette.link)
            }
        }
        .padding(14).frame(maxWidth: .infinity, alignment: .leading).surface(look, radius: 18)
    }

    internal static func clock(_ seconds: Int) -> String { "\(seconds / 60):" + String(format: "%02d", seconds % 60) }
}
