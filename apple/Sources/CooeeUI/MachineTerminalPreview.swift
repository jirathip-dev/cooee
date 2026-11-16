import SwiftUI

extension TerminalView {
    public static func agents() -> Self {
        Self(
            title: "mini", back: "Machines", tabs: ["coder · cooee", "chief · work", "zsh"],
            lines: [
                "guy@mini ~/src/cooee % cargo test -p cooee-core",
                "   Compiling cooee-core v0.9.0",
                "    Finished `test` profile in 14.2s",
                "     Running unittests src/lib.rs",
                "test result: ok. 412 passed; 0 failed",
                "guy@mini ~/src/cooee % ▍",
            ],
            actions: TerminalActions(newShell: {}))
    }

    public static func own() -> Self {
        Self(
            title: "Terminal · mini", back: "You", tabs: ["your shell · kept running"],
            lines: [
                "guy@mini ~ % git -C ~/src/cooee pull",
                "Already up to date.",
                "guy@mini ~ % df -h /",
                "/dev/disk3s1  460Gi  59Gi free  88%",
                "── reattached · scrollback above ──",
                "guy@mini ~ % ▍",
            ])
    }
}

public struct SettingsWindow: View {
    private let page: SettingsFrame

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

    public init(page: SettingsFrame) {
        self.page = page
    }
}
