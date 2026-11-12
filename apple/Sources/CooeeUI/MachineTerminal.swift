import CooeeFeatures
import SwiftUI

public struct MachineTerminal: View {
    @Environment(\.look) private var look
    private let own: Bool
    private var tabs: [String] { own ? ["your shell · kept running"] : ["coder · cooee", "chief · work", "zsh"] }
    private var lines: [String] { own ? Self.shell : Self.agent }
    private static let shell = [
        "guy@mini ~ % git -C ~/src/cooee pull",
        "Already up to date.",
        "guy@mini ~ % df -h /",
        "/dev/disk3s1  460Gi  59Gi free  88%",
        "── reattached · scrollback above ──",
        "guy@mini ~ % ▍",
    ]
    private static let agent = [
        "guy@mini ~/src/cooee % cargo test -p cooee-core",
        "   Compiling cooee-core v0.9.0",
        "    Finished `test` profile in 14.2s",
        "     Running unittests src/lib.rs",
        "test result: ok. 412 passed; 0 failed",
        "guy@mini ~/src/cooee % ▍",
    ]

    public var body: some View {
        let palette = look.palette
        VStack(spacing: 0) {
            HStack {
                Text(look.mono ? (own ? "< you" : "< machines") : (own ? "‹ You" : "‹ Machines")).ui(16, .regular, look)
                    .foregroundStyle(palette.link)
                Spacer()
                if !own { ActionPill(title: look.mono ? "+ shell" : "New shell") }
            }
            .overlay { Text(own ? "Terminal · mini" : "mini").ui(17, .semibold, look) }
            .padding(.horizontal, 16).frame(height: 44)
            strip
            Scroller {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(lines, id: \.self) { Text($0) }
                }
                .font(.system(size: 13, design: .monospaced)).foregroundStyle(Color.hex(0xd8e0c8))
                .frame(maxWidth: .infinity, alignment: .leading).padding(14)
            }
            .background(Color.hex(0x0b0f0c))
            keys
        }
        .background(palette.background).foregroundStyle(palette.foreground)
    }

    private var strip: some View {
        HStack(spacing: 8) {
            ForEach(tabs, id: \.self) { tab in
                let active = tab == tabs.first
                Text(tab).ui(13, active ? .semibold : .regular, look)
                    .foregroundStyle(active ? look.palette.foreground : look.palette.mute)
                    .padding(.horizontal, 12).frame(height: 30)
                    .background(active ? look.palette.edge.opacity(0.5) : .clear, in: Capsule())
            }
            Spacer()
        }
        .padding(.horizontal, 12).padding(.bottom, 8)
    }

    private var keys: some View {
        HStack(spacing: 6) {
            ForEach(["esc", "tab", "ctrl", "←", "↑", "↓", "→", "⌃C"], id: \.self) { key in
                Text(key).font(.system(size: 14, weight: .medium, design: .monospaced))
                    .frame(maxWidth: .infinity, minHeight: 36)
                    .background(look.palette.card, in: .rect(cornerRadius: look.mono ? 0 : 8))
            }
        }
        .padding(.horizontal, 8).padding(.top, 8).padding(.bottom, 40)
        .background(look.palette.card.opacity(0.5))
    }

    public init(own: Bool = false) {
        self.own = own
    }
}

public struct SettingsWindow: View {
    private let page: MenuFeature.Page

    public var body: some View {
        MacSettings(selected: page) {
            switch page {
            case .signIns: SignInsView(phone: false)
            case .devices: DevicesView(phone: false)
            case .machines: MachinesView(phone: false)
            default: PluginsView(phone: false)
            }
        }
    }

    public init(page: MenuFeature.Page) {
        self.page = page
    }
}
