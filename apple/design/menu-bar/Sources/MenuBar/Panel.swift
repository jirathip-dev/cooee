import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct MenuPanel: View {
    @Environment(\.look) private var look
    private let snapshot: Snapshot

    public static let width: CGFloat = 340

    public var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            StatusHeader(link: snapshot.link).padding(.horizontal, 10).padding(.top, 6)
            if case .unreachable(let name, _) = snapshot.link { Unreachable(name: name) }
            Rule()
            if case .unpaired = snapshot.link {
                NotPaired()
            } else {
                live
                Rule()
                PairQR(server: "server", dimmed: snapshot.asOf != nil)
            }
            Rule()
            MenuItem(title: "Open Cooee", keys: "⌘O", strong: true)
            MenuItem(title: "Quit Cooee", keys: "⌘Q")
        }
        .padding(6).padding(.bottom, 2).frame(width: Self.width)
        .foregroundStyle(look.palette.foreground)
        .surface(look, radius: 14)
    }

    private var live: some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 2) {
            SectionTitle(text: "Needs you", count: snapshot.asks.count, tint: palette.red)
            if snapshot.asks.isEmpty { EmptyLine(symbol: "checkmark.circle", text: "Nothing is waiting on you") }
            ForEach(snapshot.asks) { LiveLine(row: $0) }
            SectionTitle(text: "Running", count: snapshot.running.count, tint: palette.cyan).padding(.top, 8)
            if snapshot.running.isEmpty { EmptyLine(symbol: "moon.zzz", text: "No agent is running") }
            ForEach(snapshot.running) { LiveLine(row: $0) }
            if let asOf = snapshot.asOf {
                Text("as of \(asOf) · tap opens the kept copy").ui(11, .regular, look)
                    .foregroundStyle(palette.mute).padding(.horizontal, 10).padding(.top, 4)
            }
        }
        .opacity(snapshot.asOf == nil ? 1 : 0.6)
    }

    public init(_ state: PanelState) {
        snapshot = .of(state)
    }
}

internal struct Unreachable: View {
    @Environment(\.look) private var look
    internal let name: String

    internal var body: some View {
        let red = look.palette.red
        HStack(spacing: 8) {
            Image(systemName: "wifi.exclamationmark").font(.system(size: 13, weight: .semibold))
            Text("Can't reach \(name). Retrying.").ui(12, .medium, look)
            Spacer()
            Text("Retry").ui(12, .semibold, look).padding(.horizontal, 10).frame(height: 24)
                .chip(look, fill: red.opacity(0.16), edge: red)
        }
        .foregroundStyle(red).padding(.horizontal, 10).frame(height: 36)
        .accent(look, red, fill: red.opacity(0.10), radius: 8)
    }
}

internal struct NotPaired: View {
    @Environment(\.look) private var look

    internal var body: some View {
        VStack(spacing: 10) {
            Image(systemName: "link.badge.plus").font(.system(size: 26, weight: .semibold))
                .foregroundStyle(look.palette.link)
            Text("Connect this Mac to a server").ui(14, .semibold, look)
            Text("Running agents, asks and the phone's pairing QR show here once it's paired.")
                .ui(12, .regular, look).foregroundStyle(look.palette.mute).multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
            CardButton(title: "Connect…", primary: true).frame(width: 160)
        }
        .frame(maxWidth: .infinity).padding(.vertical, 14).padding(.horizontal, 16)
    }
}
