import CooeeCore
import CooeeFeatures
import SwiftUI

/// The Mac menu bar panel (#659): status, asks, running chats and the phone's pairing QR, each tap sent to `tap`.
public struct MenuPanel: View {
    public enum Tap: Sendable, Equatable {
        case chat(String)
        case main
        case retry
        case quit
    }

    public static let width: CGFloat = 340

    @Environment(\.look) private var look
    private let glance: Glance
    private let tap: (Tap) -> Void

    public var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            StatusHeader(link: glance.link).padding(.horizontal, 10).padding(.top, 6)
            if case .unreachable(let name, _) = glance.link { Unreachable(name: name) { tap(.retry) } }
            PanelRule()
            if case .unpaired = glance.link {
                NotPaired { tap(.main) }
            } else {
                live
                PanelRule()
                if let pair = glance.pair { PairQR(url: pair, dimmed: glance.stale) }
            }
            PanelRule()
            PanelItem(title: "Open Cooee", key: "o", strong: true) { tap(.main) }
            PanelItem(title: "Quit Cooee", key: "q") { tap(.quit) }
        }
        .padding(6).padding(.bottom, 2).frame(width: Self.width)
        .foregroundStyle(look.palette.foreground)
        .surface(look, radius: 14)
    }

    private var live: some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 2) {
            SectionTitle(text: "Needs you", count: glance.asks.count, tint: palette.red)
            if glance.asks.isEmpty { EmptyLine(symbol: "checkmark.circle", text: "Nothing is waiting on you") }
            lines(glance.asks, tint: palette.red)
            SectionTitle(text: "Running", count: glance.running.count, tint: palette.cyan).padding(.top, 8)
            if glance.running.isEmpty { EmptyLine(symbol: "moon.zzz", text: "No agent is running") }
            lines(glance.running, tint: palette.cyan)
            if case .unreachable(_, let since?) = glance.link {
                Text("as of \(since) · tap opens the kept copy").ui(11, .regular, look)
                    .foregroundStyle(palette.mute).padding(.horizontal, 10).padding(.top, 4)
            }
        }
        .opacity(glance.stale ? 0.6 : 1)
    }

    @ViewBuilder private func lines(_ chats: [ListedChat], tint: Color) -> some View {
        ForEach(chats.prefix(Glance.shown)) { chat in
            Button {
                tap(.chat(chat.id))
            } label: {
                LiveLine(chat: chat, tint: tint)
            }.buttonStyle(.plain)
        }
        if chats.count > Glance.shown {
            Button {
                tap(.main)
            } label: {
                MoreLine(count: chats.count - Glance.shown)
            }.buttonStyle(.plain)
        }
    }

    public init(_ glance: Glance, tap: @escaping (Tap) -> Void = { _ in }) {
        self.glance = glance
        self.tap = tap
    }
}

internal struct Unreachable: View {
    @Environment(\.look) private var look
    internal let name: String
    internal let retry: () -> Void

    internal var body: some View {
        let red = look.palette.red
        HStack(spacing: 8) {
            Image(systemName: "wifi.exclamationmark").font(.system(size: 13, weight: .semibold))
            Text("Can't reach \(name). Retrying.").ui(12, .medium, look)
            Spacer()
            Button(action: retry) {
                Text("Retry").ui(12, .semibold, look).padding(.horizontal, 10).frame(height: 24)
                    .chip(look, fill: red.opacity(0.16), edge: red)
            }
            .buttonStyle(.plain)
        }
        .foregroundStyle(red).padding(.horizontal, 10).frame(height: 36)
        .accent(look, red, fill: red.opacity(0.10), radius: 8)
    }
}

internal struct NotPaired: View {
    @Environment(\.look) private var look
    internal let connect: () -> Void

    internal var body: some View {
        VStack(spacing: 10) {
            Image(systemName: "link.badge.plus").font(.system(size: 26, weight: .semibold))
                .foregroundStyle(look.palette.link)
            Text("Connect this Mac to a server").ui(14, .semibold, look)
            Text("Running agents, asks and the phone's pairing QR show here once it's paired.")
                .ui(12, .regular, look).foregroundStyle(look.palette.mute).multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
            Button(action: connect) { CardButton(title: "Connect…", primary: true) }.buttonStyle(.plain)
                .frame(width: 160)
        }
        .frame(maxWidth: .infinity).padding(.vertical, 14).padding(.horizontal, 16)
    }
}
