import CooeeCore
import SwiftUI

/// What the widgets draw: what needs you (asks, then blocked lanes) and what runs, with its age.
public struct SurfaceGlance: Sendable {
    public let needs: [SurfaceRow]
    public let running: [SurfaceRow]
    public let counts: String
    public let rest: String
    public let asOf: String

    public var top: SurfaceRow? { needs.first }

    public init(needs: [SurfaceRow], running: [SurfaceRow], counts: String, rest: String, asOf: String) {
        self.needs = needs
        self.running = running
        self.counts = counts
        self.rest = rest
        self.asOf = asOf
    }
}

internal struct GlanceLine: View {
    @Environment(\.look) private var look
    internal let row: SurfaceRow
    internal var compact = false

    internal var body: some View {
        let palette = look.palette
        let alert = row.persona.state == .needs || row.persona.state == .fail
        HStack(spacing: 8) {
            FigureView(persona: row.persona, size: compact ? 22 : 30)
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

/// The small home widget: how many need you, the top one, and the asks and blocked counts.
public struct NeedsSmall: View {
    @Environment(\.look) private var look
    private let glance: SurfaceGlance

    public var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 4) {
            Kicker(text: "Needs you", tint: palette.red)
            Text("\(glance.needs.count)").ui(40, .bold, look)
            Text(glance.top.map { "\($0.persona.name): \($0.detail.lowercasedFirst)" } ?? "").ui(13, .medium, look)
                .lineLimit(2)
            Spacer(minLength: 0)
            Text(glance.counts).ui(12, .regular, look).foregroundStyle(palette.mute)
        }
    }

    public init(_ glance: SurfaceGlance) { self.glance = glance }
}

/// The small home widget of what runs: two rows with their clocks, then the rest.
public struct RunningSmall: View {
    @Environment(\.look) private var look
    private let glance: SurfaceGlance

    public var body: some View {
        VStack(alignment: .leading, spacing: 9) {
            Kicker(text: "Running", tint: look.palette.cyan)
            ForEach(glance.running.prefix(2)) { GlanceLine(row: $0, compact: true) }
            Spacer(minLength: 0)
            Text(glance.rest).ui(12, .regular, look).foregroundStyle(look.palette.mute)
        }
    }

    public init(_ glance: SurfaceGlance) { self.glance = glance }
}

/// The large home widget: Needs you over Running, each row opening its thread or ask, and its age.
public struct GlanceLarge: View {
    @Environment(\.look) private var look
    private let glance: SurfaceGlance
    private let opens: Bool

    public var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 10) {
            Kicker(text: "Needs you · \(glance.needs.count)", tint: palette.red)
            ForEach(glance.needs.prefix(3)) { line($0) }
            Rectangle().fill(palette.line).frame(height: 1)
            Kicker(text: "Running · \(glance.running.count)", tint: palette.cyan)
            ForEach(glance.running.prefix(3)) { line($0) }
            Spacer(minLength: 0)
            Text(glance.asOf).ui(12, .regular, look).foregroundStyle(palette.mute)
        }
    }

    @ViewBuilder private func line(_ row: SurfaceRow) -> some View {
        if opens { Link(destination: row.url) { GlanceLine(row: row) } } else { GlanceLine(row: row) }
    }

    /// `opens`: each row is a link to its thread (a still image draws none).
    public init(_ glance: SurfaceGlance, opens: Bool = true) {
        self.glance = glance
        self.opens = opens
    }
}

/// The circular lock widget: how many need you.
public struct LockCircle: View {
    @Environment(\.look) private var look
    private let count: Int

    public var body: some View {
        VStack(spacing: 0) {
            Text("\(count)").ui(24, .bold, look)
            Text("NEED").ui(9, .bold, look).foregroundStyle(look.palette.red)
        }
    }

    public init(_ count: Int) { self.count = count }
}

/// The rectangular lock widget: the top ask, which a tap opens.
public struct LockRect: View {
    @Environment(\.look) private var look
    private let row: SurfaceRow?

    public var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(look.type == .mono ? "(!) \(row?.title ?? "")" : row?.title ?? "").ui(14, .bold, look)
            Text(row?.detail ?? "Nothing needs you").ui(14, .regular, look).lineLimit(1)
            Text("Tap to open").ui(12, .medium, look).foregroundStyle(look.palette.mute)
        }
    }

    public init(_ row: SurfaceRow?) { self.row = row }
}

extension String {
    internal var lowercasedFirst: String { prefix(1).lowercased() + dropFirst() }
}
