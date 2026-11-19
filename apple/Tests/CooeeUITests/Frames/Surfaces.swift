import AppKit
import CooeeCore
@testable import CooeeUI
import SwiftUI
import Testing

private func who(_ id: String, _ state: LaneState? = nil) -> Persona {
    let persona = Fixtures.persona(id)
    return Persona(api: persona.api, look: persona.look, state: state ?? persona.state)
}

private func row(_ id: String, _ state: LaneState?, _ title: String, _ detail: String, _ trailing: String) -> SurfaceRow {
    SurfaceRow(thread: title, persona: who(id, state), title: title, detail: detail, trailing: trailing)
}

extension SurfaceGlance {
    fileprivate static let drawn = SurfaceGlance(
        needs: [
            row("res", nil, "Researcher asks", "Allow fetch on sec.gov?", "ask"),
            row("cond", .needs, "Conductor asks", "#234 design: approve?", "ask"),
            row("am", .fail, "cooee#218 · impl lane", "blocked: needs a secret", "blocked"),
        ],
        running: [
            row("coder", nil, "Coder", "Export for Kestrel", "2:14"),
            row("cond", nil, "Conductor", "cooee queue", "0:48"),
            row("chief", .working, "cooee#231 · review", "rev lane", "5:02"),
        ],
        counts: "2 asks · 1 blocked", rest: "+1 lane", asOf: "as of 9:41 · mini")
}

extension SurfaceActivity {
    fileprivate static let ask = SurfaceActivity(
        thread: "t2", persona: who("res"), headline: "Researcher asks you", place: "Kestrel filings · mini",
        clock: "12m", line: "Allow fetch on sec.gov/cgi-bin/browse-edgar for the Q3 10-Q?", foot: "Tap to open the ask",
        more: "+2 running")

    fileprivate static let running = SurfaceActivity(
        thread: "t1", persona: who("coder"), headline: "Coder", place: "Export for Kestrel · studio", clock: "2:14",
        line: "$ cargo run -p export -- --filing q3\n  wrote 412 rows to kestrel-q3.csv", foot: "working · 6 tools",
        more: "+2 running", tools: 6)
}

private struct Device<Content: View>: View {
    @Environment(\.look) private var look
    var lock = false
    var island: AnyView = AnyView(Capsule().fill(.black).frame(width: 124, height: 36))
    @ViewBuilder let content: Content

    var body: some View {
        let foreground = look.palette.foreground
        ZStack(alignment: .top) {
            content.padding(.top, 54)
            HStack {
                Text(lock ? "" : "9:41").font(.system(size: 17, weight: .semibold)).padding(.leading, 34)
                Spacer()
                HStack(spacing: 6) {
                    Image(systemName: "cellularbars")
                    Image(systemName: "wifi")
                    Image(systemName: "battery.100")
                }
                .font(.system(size: 15, weight: .semibold)).padding(.trailing, 30)
            }
            .foregroundStyle(foreground).frame(height: 54)
            island.padding(.top, 11)
            VStack {
                Spacer()
                Capsule().fill(foreground.opacity(0.85)).frame(width: 140, height: 5).padding(.bottom, 8)
            }
        }
    }
}

private struct Wallpaper: View {
    @Environment(\.look) private var look

    var body: some View {
        let palette = look.palette
        ZStack {
            palette.background
            if look.type != .mono {
                LinearGradient(
                    colors: [palette.cyan.opacity(0.35), palette.lead.opacity(0.30), palette.purple.opacity(0.30)],
                    startPoint: .topLeading, endPoint: .bottomTrailing)
            }
        }
        .padding(.top, -54)
    }
}

private struct LockScreen: View {
    @Environment(\.look) private var look

    var body: some View {
        let (palette, mono) = (look.palette, look.type == .mono)
        ZStack {
            Wallpaper()
            VStack(spacing: 6) {
                Text(mono ? "(!) 3 need you" : "Thursday 12 November  ·  3 need you").ui(17, .semibold, look)
                Text("9:41").font(.system(size: 104, weight: .bold, design: mono ? .monospaced : .rounded))
                HStack(spacing: 12) {
                    LockCircle(3).frame(width: 70, height: 70)
                        .background(Circle().strokeBorder(palette.red, lineWidth: 4).padding(2))
                        .background(palette.card.opacity(0.5), in: .circle)
                    LockRect(SurfaceGlance.drawn.top).frame(width: 168, height: 70, alignment: .leading)
                        .padding(.horizontal, 12)
                        .background(palette.card.opacity(0.5), in: .rect(cornerRadius: mono ? 0 : 18))
                }
                .padding(.top, 6)
                Spacer()
                LiveActivityCard(.ask).padding(.horizontal, 10)
                HStack {
                    RoundButton(symbol: "flashlight.off.fill", ascii: "*", size: 50)
                    Spacer()
                    RoundButton(symbol: "camera.fill", ascii: "o", size: 50)
                }
                .padding(.horizontal, 40).padding(.top, 34).padding(.bottom, 30)
            }
            .padding(.top, 14).foregroundStyle(palette.foreground)
        }
    }
}

private struct IslandScreen: View {
    @Environment(\.look) private var look

    var body: some View {
        ZStack(alignment: .top) {
            Backdrop()
            others.opacity(0.35)
            VStack(alignment: .leading, spacing: 14) {
                Kicker(text: "The island, compact")
                state(
                    "Running: the persona, then the turn's clock. Long-press opens it as above; tap opens the thread.",
                    .running)
                state(
                    "An ask outranks a turn: the asking persona and `ask`. Tap opens the ask; nothing is answered here.",
                    .ask)
                state("Ended: `replied`, for 15 minutes, then it goes.", .ended)
            }
            .padding(18).surface(look, radius: 26).padding(.horizontal, 12).padding(.top, 196)
            .foregroundStyle(look.palette.foreground)
        }
    }

    private var others: some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 14) {
            ForEach(0..<9, id: \.self) { index in
                HStack(spacing: 12) {
                    Circle().fill(palette.edge).frame(width: 40, height: 40)
                    VStack(alignment: .leading, spacing: 6) {
                        Capsule().fill(palette.edge).frame(width: CGFloat(140 + index % 3 * 40), height: 10)
                        Capsule().fill(palette.line).frame(width: CGFloat(220 - index % 4 * 30), height: 8)
                    }
                }
            }
        }
        .padding(.horizontal, 20).padding(.top, 70).frame(maxWidth: .infinity, alignment: .leading)
    }

    private func state(_ note: String, _ activity: SurfaceActivity) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                IslandMark(activity.persona)
                Spacer()
                IslandTrailing(activity)
            }
            .padding(.horizontal, 7).frame(width: 220, height: 36).background(.black, in: .capsule)
            Text(LocalizedStringKey(note)).ui(13, .regular, look).foregroundStyle(look.palette.mute)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}

extension SurfaceActivity {
    fileprivate static let ended = SurfaceActivity(
        thread: "t1", persona: who("coder", .done), headline: "Coder", place: "", clock: "", line: "", foot: "replied",
        more: "")
}

private struct Expanded: View {
    @Environment(\.look) private var look

    var body: some View {
        IslandExpanded(.running).padding(.horizontal, 20).padding(.vertical, 16).frame(width: 384)
            .background(.black, in: .rect(cornerRadius: look.type == .mono ? 0 : 44))
    }
}

private struct HomeWidgets: View {
    @Environment(\.look) private var look

    var body: some View {
        let glance = SurfaceGlance.drawn
        ZStack(alignment: .top) {
            Wallpaper()
            VStack(spacing: 16) {
                HStack(spacing: 16) {
                    NeedsSmall(glance).padding(14).frame(width: 170, height: 170, alignment: .topLeading)
                        .surface(look, radius: 24)
                    RunningSmall(glance).padding(14).frame(width: 170, height: 170, alignment: .topLeading)
                        .surface(look, radius: 24)
                }
                GlanceLarge(glance, opens: false).padding(16).frame(width: 356, height: 376, alignment: .topLeading)
                    .surface(look, radius: 26)
                Spacer()
                HStack(spacing: 24) {
                    ForEach(0..<4, id: \.self) { _ in
                        RoundedRectangle(cornerRadius: 16).fill(look.palette.card.opacity(0.8))
                            .frame(width: 62, height: 62)
                    }
                }
                .padding(14).surface(look, radius: 34).padding(.bottom, 24)
            }
            .padding(.top, 20).foregroundStyle(look.palette.foreground)
        }
    }
}

internal struct SurfaceFrame: Sendable, CustomTestStringConvertible {
    internal let name: String
    internal let palette: Palette
    internal let style: Style
    internal let view: @MainActor @Sendable () -> AnyView

    internal var testDescription: String { name }

    private static let frames: [(String, @MainActor @Sendable () -> AnyView)] = [
        ("lock", { AnyView(Device(lock: true) { LockScreen() }) }),
        ("island", { AnyView(Device(island: AnyView(Expanded())) { IslandScreen() }) }),
        ("widgets", { AnyView(Device { HomeWidgets() }) }),
    ]

    internal static let all: [SurfaceFrame] =
        [("dark", Palette.farmDark), ("light", Palette.farmLight)].flatMap { mode, palette in
            frames.map { SurfaceFrame(name: "\($0.0)-\(mode)", palette: palette, style: .bundled, view: $0.1) }
        } + [
            SurfaceFrame(name: "lock-terminal-dark", palette: .farmDark, style: .terminal, view: frames[0].1),
            SurfaceFrame(name: "island-terminal-dark", palette: .farmDark, style: .terminal, view: frames[1].1),
            SurfaceFrame(name: "widgets-scene-light", palette: .farmLight, style: .scene, view: frames[2].1),
        ]

    private static let designs = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .appendingPathComponent("design/native-surfaces")

    @MainActor
    internal func matchesItsDesign() throws {
        let content = view().frame(width: Frame.phoneWidth, height: Frame.phoneHeight).background(palette.background)
            .clipped().environment(\.look, Look(palette: palette, style: style, still: true))
            .environment(\.colorScheme, palette.dark ? .dark : .light)
        let renderer = ImageRenderer(content: content)
        renderer.scale = 2
        let file = Self.designs.appendingPathComponent("\(name).png")
        let design = try #require(NSImage(contentsOf: file)?.cgImage(forProposedRect: nil, context: nil, hints: nil))
        let got = try #require(renderer.cgImage.flatMap(Pixels.init))
        let want = try #require(Pixels(design))
        #expect(got.width == want.width && got.height == want.height)
        let off = got.differing(from: want, by: 16)
        #expect(off <= 0.025, "\(name): \(off * 100)% of pixels differ")
    }
}

@Suite(.serialized)
internal struct SurfaceFrames {
    @MainActor @Test(arguments: SurfaceFrame.all)
    internal func matchesTheDesign(_ surface: SurfaceFrame) throws {
        try surface.matchesItsDesign()
    }
}
