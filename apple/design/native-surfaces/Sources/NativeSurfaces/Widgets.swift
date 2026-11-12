import CooeeCore
@testable import CooeeUI
import SwiftUI

internal struct GlanceRow: Identifiable, Sendable {
    internal let persona: Persona
    internal let title: String
    internal let detail: String
    internal let trailing: String

    internal var id: String { title }

    internal static let needs = [
        GlanceRow(persona: .named("res"), title: "Researcher asks", detail: "Allow fetch on sec.gov?", trailing: "ask"),
        GlanceRow(
            persona: .named("cond", .needs), title: "Conductor asks", detail: "#234 design: approve?", trailing: "ask"),
        GlanceRow(
            persona: .named("am", .fail), title: "cooee#218 · impl lane", detail: "blocked: needs a secret",
            trailing: "blocked"),
    ]

    internal static let running = [
        GlanceRow(persona: .named("coder"), title: "Coder", detail: "Export for Kestrel", trailing: "2:14"),
        GlanceRow(persona: .named("cond"), title: "Conductor", detail: "cooee queue", trailing: "0:48"),
        GlanceRow(
            persona: .named("chief", .working), title: "cooee#231 · review", detail: "rev lane", trailing: "5:02"),
    ]
}

internal struct GlanceLine: View {
    @Environment(\.look) private var look
    internal let row: GlanceRow
    internal var compact = false

    internal var body: some View {
        let palette = look.palette
        let alert = row.persona.state == .needs || row.persona.state == .fail
        HStack(spacing: 8) {
            Avatar(persona: row.persona, size: compact ? 22 : 30)
            VStack(alignment: .leading, spacing: 1) {
                Text(row.title).ui(compact ? 13 : 14, .semibold, look).lineLimit(1)
                if !compact { Text(row.detail).ui(13, .regular, look).foregroundStyle(palette.mute).lineLimit(1) }
            }
            Spacer(minLength: 4)
            Text(row.trailing).ui(13, .semibold, look).monospacedDigit()
                .foregroundStyle(alert ? palette.red : palette.cyan)
        }
    }
}

internal struct NeedsSmall: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 4) {
            Kicker(text: "Needs you", tint: palette.red)
            Text("3").ui(40, .bold, look)
            Text("Researcher: allow fetch on sec.gov?").ui(13, .medium, look).lineLimit(2)
            Spacer(minLength: 0)
            Text("2 asks · 1 blocked").ui(12, .regular, look).foregroundStyle(palette.mute)
        }
        .padding(14).frame(width: 170, height: 170, alignment: .topLeading).surface(look, radius: 24)
    }
}

internal struct RunningSmall: View {
    @Environment(\.look) private var look

    internal var body: some View {
        VStack(alignment: .leading, spacing: 9) {
            Kicker(text: "Running", tint: look.palette.cyan)
            ForEach(GlanceRow.running.prefix(2)) { GlanceLine(row: $0, compact: true) }
            Spacer(minLength: 0)
            Text("+1 lane").ui(12, .regular, look).foregroundStyle(look.palette.mute)
        }
        .padding(14).frame(width: 170, height: 170, alignment: .topLeading).surface(look, radius: 24)
    }
}

internal struct GlanceLarge: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 10) {
            Kicker(text: "Needs you · 3", tint: palette.red)
            ForEach(GlanceRow.needs) { GlanceLine(row: $0) }
            Rectangle().fill(palette.line).frame(height: 1)
            Kicker(text: "Running · 3", tint: palette.cyan)
            ForEach(GlanceRow.running) { GlanceLine(row: $0) }
            Spacer(minLength: 0)
            Text("as of 9:41 · mini").ui(12, .regular, look).foregroundStyle(palette.mute)
        }
        .padding(16).frame(width: 356, height: 376, alignment: .topLeading).surface(look, radius: 26)
    }
}

public struct HomeWidgets: View {
    @Environment(\.look) private var look

    public var body: some View {
        ZStack(alignment: .top) {
            Wallpaper()
            VStack(spacing: 16) {
                HStack(spacing: 16) {
                    NeedsSmall()
                    RunningSmall()
                }
                GlanceLarge()
                Spacer()
                dock
            }
            .padding(.top, 20).foregroundStyle(look.palette.foreground)
        }
    }

    private var dock: some View {
        HStack(spacing: 24) {
            ForEach(0..<4, id: \.self) { _ in
                RoundedRectangle(cornerRadius: 16).fill(look.palette.card.opacity(0.8)).frame(width: 62, height: 62)
            }
        }
        .padding(14).surface(look, radius: 34).padding(.bottom, 24)
    }

    public init() {}
}
