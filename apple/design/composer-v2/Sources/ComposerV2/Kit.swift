import CooeeCore
@testable import CooeeUI
import SwiftUI

public enum BarState: Sendable {
    case empty, typing, slash, recording, locked
}

public enum Keyboard: Sendable {
    case none, apple, thai, cooee
}

internal enum Story {
    internal static let coder = Fixtures.persona("coder")

    internal static let items: [Item] = [
        .you(id: 1, text: "The look sheet e2e failed twice on CI. Can you find why?"),
        .tools(id: 2, rows: [ToolRow("rg settled web/e2e", "ok"), ToolRow("cargo test -p cooee-core", "ok")]),
        .agent(
            id: 3,
            markdown: [
                .text(
                    "A race: the sheet slides in while the test taps **Save**. It now waits on `sheet.settled`, and passed 20 runs in a row."
                )
            ]
        ),
    ]

    internal static let reply: Item = .agent(
        id: 5,
        markdown: [.text("On it: 50 runs of `look.spec` on CI, then the flake rate here.")]
    )

    internal static let typed = "Rerun it 50 times on CI\nand post the flake rate"
}

internal struct Glyph: View {
    @Environment(\.look) private var look
    internal let symbol: String
    internal let ascii: String
    internal var size: CGFloat = 17

    internal var body: some View {
        if look.mono {
            Text(ascii).ui(size - 2, .bold, look)
        } else {
            Image(systemName: symbol).font(.system(size: size, weight: .semibold))
        }
    }
}

internal struct Grabber: View {
    @Environment(\.look) private var look

    internal var body: some View {
        if look.mono {
            Text("── ── ──").ui(11, .bold, look).foregroundStyle(look.palette.edge)
        } else {
            Capsule().fill(look.palette.mute.opacity(0.5)).frame(width: 38, height: 5)
        }
    }
}

internal struct Sheet<Content: View>: View {
    @Environment(\.look) private var look
    @ViewBuilder internal var content: Content

    internal var body: some View {
        let palette = look.palette
        VStack(spacing: 0) {
            Grabber().padding(.top, 8).padding(.bottom, 10)
            content
        }
        .frame(maxWidth: .infinity)
        .background(sheetFill(palette))
    }

    @ViewBuilder private func sheetFill(_ palette: Palette) -> some View {
        switch look.style {
        case .terminal:
            Rectangle().fill(palette.background)
                .overlay(Rectangle().strokeBorder(palette.edge, style: StrokeStyle(lineWidth: 1, dash: [3, 2])))
        case .scene:
            UnevenRoundedRectangle(topLeadingRadius: 22, topTrailingRadius: 22).fill(palette.card)
                .overlay(
                    UnevenRoundedRectangle(topLeadingRadius: 22, topTrailingRadius: 22)
                        .strokeBorder(palette.edge, lineWidth: 1.5)
                )
        case .modern:
            UnevenRoundedRectangle(topLeadingRadius: 34, topTrailingRadius: 34)
                .fill(palette.dark ? Color.hex(0x1c2119) : Color.hex(0xfbf7ea))
                .shadow(color: .black.opacity(0.25), radius: 20, y: -4)
        }
    }
}

internal struct Dim: View {
    @Environment(\.look) private var look

    internal var body: some View {
        Color.black.opacity(look.palette.dark ? 0.5 : 0.28).padding(.top, -54)
    }
}
