import CooeeCore
import SwiftUI

internal struct SyncBadge: View {
    @Environment(\.look) private var look
    internal let sync: Sync
    internal var asOf: Date?

    internal var body: some View {
        HStack(spacing: 5) {
            Circle().fill(colour).frame(width: 7, height: 7)
            TimelineView(.everyMinute) { context in
                if let line = Self.line(sync, asOf: asOf, now: context.date) {
                    Text(line).ui(12, .medium, look).foregroundStyle(look.palette.mute).lineLimit(1)
                }
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(Self.line(sync, asOf: asOf, now: Date()) ?? "Live")
    }

    private var colour: Color {
        switch sync {
        case .live: look.palette.green
        case .reconnecting: look.palette.amber
        case .offline: look.palette.mute
        }
    }

    internal static func word(_ sync: Sync) -> String? {
        switch sync {
        case .live: nil
        case .reconnecting: "Reconnecting…"
        case .offline: "Offline"
        }
    }

    private static func line(_ sync: Sync, asOf: Date?, now: Date) -> String? {
        let kept = asOf.map { kept in
            let age = Relative.text(now.timeIntervalSince(kept))
            return age == "now" ? "as of now" : "as of \(age) ago"
        }
        let parts = [word(sync)?.lowercased(), kept].compactMap(\.self)
        return parts.isEmpty ? nil : parts.joined(separator: " · ")
    }
}

internal struct ToastView: View {
    @Environment(\.look) private var look
    internal let text: String

    internal var body: some View {
        Text(text).ui(13, .medium, look).foregroundStyle(look.palette.foreground).lineLimit(2)
            .padding(.horizontal, 14).padding(.vertical, 10).surface(look, radius: 16)
            .padding(12).transition(.opacity).allowsHitTesting(false)
    }
}

internal struct SlowStrip: View {
    @Environment(\.look) private var look
    @State private var shown = false

    internal var body: some View {
        LoadingBar().opacity(shown || look.still ? 1 : 0)
            .onAppear { withAnimation(.easeIn(duration: 0.2).delay(0.3)) { shown = true } }
    }
}

internal struct Toasting: ViewModifier {
    internal let toast: String?

    internal func body(content: Content) -> some View {
        content
            .overlay(alignment: .top) { if let toast { ToastView(text: toast).padding(.top, 52) } }
            .animation(.snappy, value: toast)
    }
}
