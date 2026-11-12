import CooeeCore
@testable import CooeeUI
import SwiftUI

public enum Dictation: Sendable {
    case listening, edit
}

public struct DictationScreen: View {
    @Environment(\.look) private var look
    private let stage: Dictation
    private let coder = Persona.named("coder", .done)

    private static let items: [Item] = [
        .you(id: 1, text: "Export Kestrel's filings to CSV, one row a line item."),
        .tools(id: 2, rows: [ToolRow("cargo run -p export", "ok"), ToolRow("wrote kestrel-q2.csv", "ok")]),
        .agent(id: 3, markdown: [.text("Done: **388 rows** in `kestrel-q2.csv`. The Q3 filing isn't in yet.")]),
    ]

    public var body: some View {
        ZStack {
            Backdrop()
            VStack(spacing: 0) {
                ChatHeader(persona: coder, chips: [])
                VStack(alignment: .leading, spacing: 14) {
                    ForEach(Self.items) { ItemView(item: $0) }
                }
                .padding(.horizontal, 16).padding(.top, 8)
                Spacer()
                DictationComposer(stage: stage).padding(.horizontal, 12).padding(.bottom, 26)
            }
        }
        .foregroundStyle(look.palette.foreground)
    }

    public init(_ stage: Dictation) {
        self.stage = stage
    }
}

internal struct DictationComposer: View {
    @Environment(\.look) private var look
    internal let stage: Dictation

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 8) {
            if stage == .edit { alternatives }
            HStack(alignment: .bottom, spacing: 8) {
                RoundButton(symbol: "plus", ascii: "+", size: 42)
                field
                send
            }
            HStack(spacing: 6) {
                StateMark(state: stage == .listening ? .needs : .done)
                Text(stage == .listening ? "Listening · on this iPhone" : "Dictated · edit it, then send")
            }
            .ui(12, .medium, look).foregroundStyle(palette.mute).padding(.leading, 56)
        }
    }

    private var field: some View {
        let palette = look.palette
        return HStack(alignment: .top, spacing: 8) {
            VStack(alignment: .leading, spacing: 8) {
                if stage == .listening { Wave() }
                words.ui(16, .regular, look).fixedSize(horizontal: false, vertical: true)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            mic
        }
        .padding(.leading, 16).padding(.trailing, 8).padding(.vertical, 8)
        .surface(look, radius: 21)
        .foregroundStyle(palette.foreground)
    }

    private var words: Text {
        let palette = look.palette
        if stage == .listening {
            return Text("Rerun the export with the Q3 \(Text("ten Q filing ▍").foregroundStyle(palette.mute))")
        }
        return Text("Rerun the export with the Q3 \(Text("ten Q").underline(color: palette.amber)) filing")
    }

    @ViewBuilder private var mic: some View {
        let palette = look.palette
        if stage == .listening {
            Image(systemName: "stop.fill").font(.system(size: 11)).foregroundStyle(.white)
                .frame(width: 26, height: 26).background(
                    palette.red, in: look.mono ? AnyShape(Rectangle()) : AnyShape(Circle()))
        } else {
            Image(systemName: "mic").foregroundStyle(palette.mute).frame(width: 26, height: 26)
        }
    }

    private var send: some View {
        let palette = look.palette
        return Image(systemName: "arrow.up").font(.system(size: 17, weight: .bold))
            .foregroundStyle(palette.dark ? Color.black : .white).frame(width: 42, height: 42)
            .background(palette.link, in: look.mono ? AnyShape(Rectangle()) : AnyShape(Circle()))
            .opacity(stage == .listening ? 0.35 : 1)
    }

    private var alternatives: some View {
        HStack(spacing: 6) {
            Text("ten Q →").ui(12, .regular, look).foregroundStyle(look.palette.mute)
            ForEach(["10-Q", "10Q", "tenkey"], id: \.self) { word in
                Text(word).ui(13, .semibold, look).foregroundStyle(look.palette.link)
                    .padding(.horizontal, 12).frame(height: 30).modifier(RoundSurface(look: look))
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
