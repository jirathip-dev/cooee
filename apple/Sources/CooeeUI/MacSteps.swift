import CooeeCore
import SwiftUI

internal struct MacWelcome: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 16) {
            AppMark(size: 96)
            Text("Welcome to cooee").ui(30, .bold, look)
            Text(
                "This Mac runs your agents. Your iPhone, iPad and other Macs talk to them over your own network: nothing goes through a cooee cloud."
            )
            .ui(15, .regular, look).foregroundStyle(palette.mute).frame(maxWidth: 480, alignment: .leading)
            Spacer()
            HStack {
                Text("Only a client?").ui(13, .regular, look).foregroundStyle(palette.mute)
                Text("Connect to a server elsewhere").ui(13, .regular, look).foregroundStyle(palette.link)
                Spacer()
                MacButton(title: "Set up this Mac", primary: true)
            }
        }
    }
}

internal struct MacFolders: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 14) {
            Text("Folders & agents").ui(26, .bold, look)
            Text("Agents work only inside these folders.").ui(14, .regular, look).foregroundStyle(palette.mute)
            MacList {
                MacRow(icon: "folder.fill", title: "~/work", detail: "")
                MacRow(icon: "folder.fill", title: "~/src", detail: "")
                MacRow(icon: "plus", title: "Add a folder…", detail: "", link: true, last: true)
            }
            Text("AGENTS FOUND").ui(11, .semibold, look).foregroundStyle(palette.mute).padding(.top, 8)
            MacList {
                MacRow(
                    icon: "checkmark.circle.fill", title: "Claude Code", detail: "2.4.1 · /opt/homebrew/bin/claude",
                    ok: true
                )
                MacRow(icon: "checkmark.circle.fill", title: "Codex", detail: "0.61 · ~/.local/bin/codex", ok: true)
                MacRow(icon: "circle.dashed", title: "Hermes", detail: "not installed · optional", last: true)
            }
            Spacer()
            MacNav()
        }
    }
}

internal struct MacSignins: View {
    @Environment(\.look) private var look

    internal var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Sign-ins").ui(26, .bold, look)
            Text("Each agent signs in on this Mac, with its own account. cooee never sees a password.")
                .ui(14, .regular, look).foregroundStyle(look.palette.mute)
            MacList {
                MacRow(
                    icon: "checkmark.circle.fill", title: "Claude Code", detail: "guy@… · Max plan", ok: true,
                    trailing: "Sign out"
                )
                MacRow(
                    icon: "exclamationmark.circle", title: "Codex", detail: "not signed in", trailing: "Sign in…",
                    last: true
                )
            }
            Spacer()
            MacNav()
        }
    }
}

internal struct MacTailscale: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 14) {
            Text("Reach it from anywhere").ui(26, .bold, look)
            Text("On this Wi-Fi your phone finds the Mac by itself. Away from home, cooee uses Tailscale.")
                .ui(14, .regular, look).foregroundStyle(palette.mute)
            missing
            MacList {
                MacRow(icon: "checkmark.circle.fill", title: "Wi-Fi", detail: "server.local · 192.0.2.20", ok: true)
                MacRow(icon: "circle.dashed", title: "Tailscale", detail: "not found", last: true)
            }
            Spacer()
            HStack {
                Spacer()
                MacButton(title: "Wi-Fi only for now")
                MacButton(title: "Continue", primary: true)
            }
        }
    }

    private var missing: some View {
        let palette = look.palette
        return HStack(alignment: .top, spacing: 14) {
            Image(systemName: "exclamationmark.triangle.fill").font(.system(size: 22)).foregroundStyle(palette.amber)
            VStack(alignment: .leading, spacing: 6) {
                Text("Tailscale isn't installed").ui(16, .semibold, look)
                Text(
                    "Install it from tailscale.com and sign in on this Mac and your phone; then Check again. You can skip this and pair on Wi-Fi only."
                )
                .ui(13, .regular, look).foregroundStyle(palette.mute).fixedSize(horizontal: false, vertical: true)
                HStack {
                    MacButton(title: "Get Tailscale…")
                    MacButton(title: "Check again")
                }
                .padding(.top, 4)
            }
        }
        .padding(16).surface(look, radius: 14)
    }
}

internal struct MacPairPhone: View {
    @Environment(\.look) private var look

    internal var body: some View {
        let palette = look.palette
        HStack(alignment: .center, spacing: 36) {
            VStack(spacing: 10) {
                QRCode(text: "https://server.example.com/pair?c=K7Q2MX", size: 230)
                    .padding(18).background(.white, in: .rect(cornerRadius: 18))
                Text("or enter K7Q2MX").font(.system(size: 13, design: .monospaced)).foregroundStyle(palette.mute)
            }
            VStack(alignment: .leading, spacing: 12) {
                Text("Pair your iPhone").ui(26, .bold, look)
                Text("Open cooee on the phone and tap Scan QR. Or pick mini under Nearby, or type its address:")
                    .ui(14, .regular, look).foregroundStyle(palette.mute).fixedSize(horizontal: false, vertical: true)
                Text("server.example.com").font(.system(size: 14, design: .monospaced)).padding(8)
                    .background(palette.card, in: .rect(cornerRadius: 8))
                HStack(spacing: 8) {
                    ProgressDots()
                    Text("Waiting for a phone · code lasts 5 min").ui(12, .regular, look)
                        .foregroundStyle(palette.mute)
                }
                Text("`cooee setup` in a terminal prints this same QR.").ui(12, .regular, look)
                    .foregroundStyle(palette.mute)
                Spacer().frame(height: 10)
                MacButton(title: "Done")
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
