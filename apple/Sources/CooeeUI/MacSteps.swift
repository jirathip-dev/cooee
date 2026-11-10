import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct MacWelcome: View {
    @Environment(\.look) private var look
    internal let send: MacSend

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
                Button {
                    send(.connectElsewhere)
                } label: {
                    Text("Connect to a server elsewhere").ui(13, .regular, look).foregroundStyle(palette.link)
                }
                .buttonStyle(.plain)
                Spacer()
                MacButton(title: "Set up this Mac", primary: true) { send(.setUp) }
            }
        }
    }
}

internal struct MacFolders: View {
    @Environment(\.look) private var look
    internal let state: MacSetupFeature.State
    internal let send: MacSend

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 14) {
            Text("Folders & agents").ui(26, .bold, look)
            Text("Agents work only inside these folders.").ui(14, .regular, look).foregroundStyle(palette.mute)
            MacList {
                ForEach(state.folders, id: \.self) { MacRow(icon: "folder.fill", title: $0, detail: "") }
                MacRow(icon: "plus", title: "Add a folder…", detail: "", link: true, last: true, action: pick)
            }
            Text("AGENTS FOUND").ui(11, .semibold, look).foregroundStyle(palette.mute).padding(.top, 8)
            MacList {
                ForEach(state.agents, id: \.name) { agent in
                    MacRow(
                        icon: agent.path == nil ? "circle.dashed" : "checkmark.circle.fill", title: agent.title,
                        detail: agent.detail, done: agent.path != nil, last: agent == state.agents.last
                    )
                }
            }
            Spacer()
            MacNav(send: send)
        }
    }

    private func pick() {
        #if os(macOS)
            let panel = NSOpenPanel()
            panel.canChooseFiles = false
            panel.canChooseDirectories = true
            if panel.runModal() == .OK, let url = panel.url { send(.addFolder(url.path(percentEncoded: false))) }
        #endif
    }
}

internal struct MacSignins: View {
    @Environment(\.look) private var look
    internal let state: MacSetupFeature.State
    internal let send: MacSend

    internal var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Sign-ins").ui(26, .bold, look)
            Text("Each agent signs in on this Mac, with its own account. cooee never sees a password.")
                .ui(14, .regular, look).foregroundStyle(look.palette.mute)
            MacList {
                ForEach(state.signIns, id: \.provider) { signIn in
                    MacRow(
                        icon: signIn.signedIn ? "checkmark.circle.fill" : "exclamationmark.circle",
                        title: FoundAgent.title(signIn.provider), detail: signIn.detail, done: signIn.signedIn,
                        trailing: signIn.signedIn ? "Sign out" : "Sign in…", last: signIn == state.signIns.last
                    ) { send(.signIn(signIn.provider)) }
                }
            }
            Spacer()
            MacNav(send: send)
        }
    }
}

internal struct MacTailscale: View {
    @Environment(\.look) private var look
    @Environment(\.openURL) private var openURL
    internal let state: MacSetupFeature.State
    internal let send: MacSend

    internal var body: some View {
        let palette = look.palette
        let tailnet = state.network?.tailnet
        VStack(alignment: .leading, spacing: 14) {
            Text("Reach it from anywhere").ui(26, .bold, look)
            Text("On this Wi-Fi your phone finds the Mac by itself. Away from home, cooee uses Tailscale.")
                .ui(14, .regular, look).foregroundStyle(palette.mute)
            if state.network != nil && tailnet == nil { missing }
            MacList {
                MacRow(icon: "checkmark.circle.fill", title: "Wi-Fi", detail: state.network?.lan ?? "", done: true)
                MacRow(
                    icon: tailnet == nil ? "circle.dashed" : "checkmark.circle.fill", title: "Tailscale",
                    detail: tailnet ?? (state.network == nil ? "checking…" : "not found"), done: tailnet != nil,
                    last: true
                )
            }
            Spacer()
            HStack {
                Spacer()
                MacButton(title: "Wi-Fi only for now") { send(.next) }
                MacButton(title: "Continue", primary: true) { send(.next) }
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
                    MacButton(title: "Get Tailscale…") {
                        if let url = URL(string: "https://tailscale.com/download/mac") { openURL(url) }
                    }
                    MacButton(title: "Check again") { send(.checkNetwork) }
                }
                .padding(.top, 4)
            }
        }
        .padding(16).surface(look, radius: 14)
    }
}

internal struct MacPairPhone: View {
    @Environment(\.look) private var look
    internal let state: MacSetupFeature.State
    internal let send: MacSend

    internal var body: some View {
        let palette = look.palette
        HStack(alignment: .center, spacing: 36) {
            VStack(spacing: 10) {
                QRCode(text: state.code?.link.absoluteString ?? "", size: 230)
                    .padding(18).background(.white, in: .rect(cornerRadius: 18))
                Text("or enter \(state.code?.code ?? "…")").font(.system(size: 13, design: .monospaced))
                    .foregroundStyle(palette.mute)
            }
            VStack(alignment: .leading, spacing: 12) {
                Text("Pair your iPhone").ui(26, .bold, look)
                Text(
                    "Open cooee on the phone and tap Scan QR. Or pick \(state.network?.name ?? "this Mac") under Nearby, or type its address:"
                )
                .ui(14, .regular, look).foregroundStyle(palette.mute).fixedSize(horizontal: false, vertical: true)
                Text(state.code?.host ?? "…").font(.system(size: 14, design: .monospaced)).padding(8)
                    .background(palette.card, in: .rect(cornerRadius: 8))
                HStack(spacing: 8) {
                    ProgressDots()
                    Text(waiting).ui(12, .regular, look).foregroundStyle(palette.mute)
                }
                Text("`cooee setup` in a terminal prints this same QR.").ui(12, .regular, look)
                    .foregroundStyle(palette.mute)
                Spacer().frame(height: 10)
                MacButton(title: "Done") { send(.done) }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var waiting: String {
        let approved = state.approved == 0 ? "" : "\(state.approved) approved · "
        return "\(approved)Waiting for a phone · code lasts \(state.code?.minutes ?? 10) min"
    }
}
