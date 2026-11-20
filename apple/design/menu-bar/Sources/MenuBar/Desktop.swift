import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct MenuBarIcon: View {
    @Environment(\.look) private var look
    private let state: PanelState
    private let open: Bool

    public var body: some View {
        let asks = Snapshot.of(state).asks.count
        HStack(spacing: 3) {
            Image(systemName: "bird.fill").font(.system(size: 14, weight: .semibold))
            if asks > 0, state != .unreachable {
                Text("\(asks)").font(.system(size: 13, weight: .semibold)).monospacedDigit()
            }
            if state == .unreachable { Image(systemName: "exclamationmark").font(.system(size: 10, weight: .heavy)) }
        }
        .opacity(state == .unpaired ? 0.45 : 1)
        .padding(.horizontal, 8).frame(height: 22)
        .background(open ? Color.primary.opacity(0.18) : .clear, in: .rect(cornerRadius: 5))
    }

    public init(_ state: PanelState, open: Bool = true) {
        self.state = state
        self.open = open
    }
}

internal struct MenuBarStrip: View {
    @Environment(\.look) private var look
    internal let state: PanelState

    internal var body: some View {
        HStack(spacing: 18) {
            Image(systemName: "apple.logo").font(.system(size: 15, weight: .semibold))
            Text("Finder").font(.system(size: 13, weight: .bold))
            ForEach(["File", "Edit", "View", "Go", "Window", "Help"], id: \.self) {
                Text($0).font(.system(size: 13))
            }
            Spacer()
            HStack(spacing: 6) {
                MenuBarIcon(state)
                Image(systemName: "battery.75").frame(width: 30)
                Image(systemName: "wifi").frame(width: 30)
                Image(systemName: "magnifyingglass").frame(width: 30)
                Text("Fri 20 Nov  9:41").font(.system(size: 13)).padding(.leading, 6)
            }
        }
        .foregroundStyle(look.palette.dark ? Color.white : Color.black)
        .padding(.horizontal, 16).frame(height: 30)
        .background(look.palette.dark ? Color.black.opacity(0.35) : Color.white.opacity(0.45))
    }
}

public struct Desktop: View {
    @Environment(\.look) private var look
    private let state: PanelState

    public var body: some View {
        let palette = look.palette
        ZStack(alignment: .topTrailing) {
            LinearGradient(
                colors: [palette.cyan.opacity(0.45), palette.lead.opacity(0.35), palette.purple.opacity(0.40)],
                startPoint: .topLeading, endPoint: .bottomTrailing
            )
            .background(palette.background)
            MenuBarStrip(state: state).frame(maxWidth: .infinity)
            MenuPanel(state).shadow(color: .black.opacity(0.3), radius: 24, y: 10)
                .padding(.top, 34).padding(.trailing, 196)
        }
        .frame(width: 1440, height: 900)
    }

    public init(_ state: PanelState) {
        self.state = state
    }
}

public struct IconStates: View {
    @Environment(\.look) private var look

    public var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            ForEach(PanelState.allCases, id: \.self) { state in
                HStack {
                    MenuBarIcon(state, open: false).frame(width: 90, alignment: .leading)
                    Text(words(state)).ui(13, .regular, look).foregroundStyle(look.palette.mute)
                    Spacer()
                }
            }
        }
        .foregroundStyle(look.palette.foreground).padding(20).frame(width: 390)
        .background(look.palette.background)
    }

    private func words(_ state: PanelState) -> String {
        switch state {
        case .running: "asks waiting: their count"
        case .quiet: "nothing waiting: the mark alone"
        case .unreachable: "server not reachable: a mark"
        case .unpaired: "not paired: dimmed"
        }
    }

    public init() {}
}
