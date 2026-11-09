import SwiftUI

/// The Mac app's first run as the server (q962 option 1: a notarized download that runs cooee and shed as login
/// items): welcome, folders and agents, sign-ins, a Tailscale check, then the QR code, and Approve when a phone asks.
public enum MacStep: String, CaseIterable, Sendable { case welcome, folders, signins, tailscale, qr, approve }

public struct MacSetup: View {
    @Environment(\.look) var look
    let step: MacStep
    public init(step: MacStep) { self.step = step }
    static let steps: [(MacStep, String, String)] = [
        (.welcome, "Welcome", "hand.wave"), (.folders, "Folders & agents", "folder"), (.signins, "Sign-ins", "person.badge.key"),
        (.tailscale, "Tailscale", "network"), (.qr, "Pair a phone", "qrcode"),
    ]
    public var body: some View {
        let p = look.palette
        HStack(spacing: 0) {
            // The steps, as a sidebar.
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 8) { AppMark(size: 34); Text("cooee setup").ui(15, .bold, look) }.padding(.bottom, 14)
                ForEach(Self.steps, id: \.0) { s, title, icon in
                    let idx = Self.steps.firstIndex { $0.0 == s }!, cur = Self.steps.firstIndex { $0.0 == (step == .approve ? .qr : step) }!
                    HStack(spacing: 10) {
                        Image(systemName: idx < cur ? "checkmark.circle.fill" : icon).frame(width: 18)
                            .foregroundStyle(idx < cur ? p.g : (idx == cur ? p.l : p.mute))
                        Text(title).ui(13, idx == cur ? .semibold : .regular, look).foregroundStyle(idx > cur ? p.mute : p.fg)
                    }
                    .padding(.horizontal, 10).frame(height: 32).frame(maxWidth: .infinity, alignment: .leading)
                    .background(idx == cur ? p.link.opacity(0.16) : .clear, in: .rect(cornerRadius: 8))
                }
                Spacer()
                Text("cooee and shed run as login items. Quit from the menu bar.").ui(11, .regular, look).foregroundStyle(p.mute)
            }
            .padding(16).padding(.top, 30).frame(width: 220).background(p.card.opacity(0.55))
            ZStack {
                Backdrop()
                content.padding(36).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    .blur(radius: step == .approve ? 8 : 0)
                if step == .approve {
                    p.bg.opacity(0.5)
                    ApprovePrompt()
                }
            }
        }
        .foregroundStyle(p.fg)
    }

    @ViewBuilder var content: some View {
        let p = look.palette
        switch step {
        case .welcome:
            VStack(alignment: .leading, spacing: 16) {
                AppMark(size: 96)
                Text("Welcome to cooee").ui(30, .bold, look)
                Text("This Mac runs your agents. Your iPhone, iPad and other Macs talk to them over your own network: nothing goes through a cooee cloud.")
                    .ui(15, .regular, look).foregroundStyle(p.mute).frame(maxWidth: 480, alignment: .leading)
                Spacer()
                HStack {
                    Text("Only a client?").ui(13, .regular, look).foregroundStyle(p.mute)
                    Text("Connect to a server elsewhere").ui(13, .regular, look).foregroundStyle(p.link)
                    Spacer()
                    MacButton(title: "Set up this Mac", primary: true)
                }
            }
        case .folders:
            VStack(alignment: .leading, spacing: 14) {
                Text("Folders & agents").ui(26, .bold, look)
                Text("Agents work only inside these folders.").ui(14, .regular, look).foregroundStyle(p.mute)
                MacList {
                    MacRow(icon: "folder.fill", title: "~/work", detail: "")
                    MacRow(icon: "folder.fill", title: "~/src", detail: "")
                    MacRow(icon: "plus", title: "Add a folder…", detail: "", link: true, last: true)
                }
                Text("AGENTS FOUND").ui(11, .semibold, look).foregroundStyle(p.mute).padding(.top, 8)
                MacList {
                    MacRow(icon: "checkmark.circle.fill", title: "Claude Code", detail: "2.4.1 · /opt/homebrew/bin/claude", ok: true)
                    MacRow(icon: "checkmark.circle.fill", title: "Codex", detail: "0.61 · ~/.local/bin/codex", ok: true)
                    MacRow(icon: "circle.dashed", title: "Hermes", detail: "not installed · optional", last: true)
                }
                Spacer()
                nav
            }
        case .signins:
            VStack(alignment: .leading, spacing: 14) {
                Text("Sign-ins").ui(26, .bold, look)
                Text("Each agent signs in on this Mac, with its own account. cooee never sees a password.").ui(14, .regular, look).foregroundStyle(p.mute)
                MacList {
                    MacRow(icon: "checkmark.circle.fill", title: "Claude Code", detail: "guy@… · Max plan", ok: true, trailing: "Sign out")
                    MacRow(icon: "exclamationmark.circle", title: "Codex", detail: "not signed in", trailing: "Sign in…", last: true)
                }
                Spacer()
                nav
            }
        case .tailscale:
            VStack(alignment: .leading, spacing: 14) {
                Text("Reach it from anywhere").ui(26, .bold, look)
                Text("On this Wi-Fi your phone finds the Mac by itself. Away from home, cooee uses Tailscale.").ui(14, .regular, look).foregroundStyle(p.mute)
                HStack(alignment: .top, spacing: 14) {
                    Image(systemName: "exclamationmark.triangle.fill").font(.system(size: 22)).foregroundStyle(p.a)
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Tailscale isn't installed").ui(16, .semibold, look)
                        Text("Install it from tailscale.com and sign in on this Mac and your phone; then Check again. You can skip this and pair on Wi-Fi only.")
                            .ui(13, .regular, look).foregroundStyle(p.mute).fixedSize(horizontal: false, vertical: true)
                        HStack { MacButton(title: "Get Tailscale…"); MacButton(title: "Check again") }.padding(.top, 4)
                    }
                }
                .padding(16).surface(look, radius: 14)
                MacList {
                    MacRow(icon: "checkmark.circle.fill", title: "Wi-Fi", detail: "mini.local · 192.168.1.20", ok: true)
                    MacRow(icon: "circle.dashed", title: "Tailscale", detail: "not found", last: true)
                }
                Spacer()
                HStack { Spacer(); MacButton(title: "Wi-Fi only for now"); MacButton(title: "Continue", primary: true) }
            }
        case .qr, .approve:
            HStack(alignment: .center, spacing: 36) {
                VStack(spacing: 10) {
                    QRCode(text: "https://mini.tail4a2b.ts.net/pair?c=K7Q2MX", size: 230)
                        .padding(18).background(.white, in: .rect(cornerRadius: 18))
                    Text("or enter K7Q2MX").font(.system(size: 13, design: .monospaced)).foregroundStyle(p.mute)
                }
                VStack(alignment: .leading, spacing: 12) {
                    Text("Pair your iPhone").ui(26, .bold, look)
                    Text("Open cooee on the phone and tap Scan QR. Or pick mini under Nearby, or type its address:")
                        .ui(14, .regular, look).foregroundStyle(p.mute).fixedSize(horizontal: false, vertical: true)
                    Text("mini.tail4a2b.ts.net").font(.system(size: 14, design: .monospaced)).padding(8)
                        .background(p.card, in: .rect(cornerRadius: 8))
                    HStack(spacing: 8) { ProgressDots(); Text("Waiting for a phone · code lasts 5 min").ui(12, .regular, look).foregroundStyle(p.mute) }
                    Text("`cooee setup` in a terminal prints this same QR.").ui(12, .regular, look).foregroundStyle(p.mute)
                    Spacer().frame(height: 10)
                    MacButton(title: "Done")
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    var nav: some View { HStack { MacButton(title: "Back"); Spacer(); MacButton(title: "Continue", primary: true) } }
}

/// A phone asks to pair: the Mac shows the check word, and Approve needs Touch ID.
struct ApprovePrompt: View {
    @Environment(\.look) var look
    var body: some View {
        let p = look.palette
        VStack(spacing: 12) {
            Image(systemName: "iphone.gen3.radiowaves.left.and.right").font(.system(size: 36)).foregroundStyle(p.link)
            Text("Guy's iPhone wants to pair").ui(17, .bold, look)
            Text("Approve only if it shows the same word:").ui(13, .regular, look).foregroundStyle(p.mute)
            Text("maple").font(.system(size: 30, weight: .bold, design: .monospaced)).tracking(3).foregroundStyle(p.l)
            Text("from 192.168.1.44 · just now").ui(12, .regular, look).foregroundStyle(p.mute)
            HStack(spacing: 10) {
                MacButton(title: "Decline").frame(maxWidth: .infinity)
                MacButton(title: "Approve · Touch ID", primary: true).frame(maxWidth: .infinity)
            }
            .padding(.top, 6)
        }
        .padding(24).frame(width: 380).surface(look, radius: 24)
    }
}

/// The Mac app as a client only: connect to a server someone runs elsewhere.
public struct MacConnect: View {
    @Environment(\.look) var look
    public init() {}
    public var body: some View {
        let p = look.palette
        ZStack {
            Backdrop()
            VStack(spacing: 18) {
                AppMark(size: 80)
                Text("Connect to a server").ui(26, .bold, look)
                Text("cooee runs on another Mac or a Linux box. This Mac only talks to it.").ui(14, .regular, look).foregroundStyle(p.mute)
                HStack(alignment: .top, spacing: 18) {
                    VStack(alignment: .leading, spacing: 10) {
                        Text("NEARBY").ui(11, .semibold, look).foregroundStyle(p.mute)
                        NearbyList()
                    }
                    VStack(alignment: .leading, spacing: 10) {
                        Text("ADDRESS").ui(11, .semibold, look).foregroundStyle(p.mute)
                        AddressField()
                        HStack(spacing: 8) {
                            Image(systemName: "qrcode.viewfinder").foregroundStyle(p.link)
                            Text("Scan its QR with this Mac's camera").ui(14, .regular, look).foregroundStyle(p.link)
                        }
                        .padding(.top, 8)
                    }
                }
                .frame(maxWidth: 760)
                Spacer()
                Text("Want this Mac to be the server? Set it up instead").ui(13, .regular, look).foregroundStyle(p.link)
            }
            .padding(36)
        }
        .foregroundStyle(p.fg)
    }
}

struct MacButton: View {
    @Environment(\.look) var look
    let title: String
    var primary = false
    var body: some View {
        let p = look.palette
        Text(title).ui(13, primary ? .semibold : .regular, look)
            .foregroundStyle(primary ? (p.dark ? Color.black : .white) : p.fg)
            .padding(.horizontal, 16).frame(height: 30)
            .background(primary ? p.link : p.edge.opacity(0.45), in: .capsule)
    }
}

struct MacList<C: View>: View {
    @Environment(\.look) var look
    @ViewBuilder var content: C
    var body: some View {
        VStack(spacing: 0) { content }.background(look.palette.card.opacity(0.75), in: .rect(cornerRadius: 12))
    }
}

struct MacRow: View {
    @Environment(\.look) var look
    let icon: String, title: String, detail: String
    var ok = false, link = false
    var trailing = ""
    var last = false
    var body: some View {
        let p = look.palette
        HStack(spacing: 10) {
            Image(systemName: icon).foregroundStyle(ok ? p.g : (link ? p.link : p.mute)).frame(width: 20)
            Text(title).ui(14, .regular, look).foregroundStyle(link ? p.link : p.fg)
            Text(detail).ui(12, .regular, look).foregroundStyle(p.mute)
            Spacer()
            if !trailing.isEmpty { MacButton(title: trailing, primary: trailing.hasSuffix("…")) }
        }
        .padding(.horizontal, 12).frame(height: 40)
        .overlay(alignment: .bottom) { if !last { Rectangle().fill(p.line).frame(height: 0.5).padding(.leading, 42) } }
    }
}

/// Mac and iPad: the home's chat list is the sidebar beside the open chat.
public struct WideView: View {
    @Environment(\.look) var look
    public init() {}
    public var body: some View {
        let p = look.palette
        HStack(spacing: 0) {
            VStack(spacing: 0) {
                TopBar().padding(.top, 34)
                Scroller {
                    PinnedRow().scaleEffect(0.9, anchor: .leading).padding(.top, 4)
                    ChatList(selected: "t1").padding(.top, 8)
                }
            }
            .frame(width: 360).background(p.card.opacity(look.style == .modern ? 0.45 : 0.8))
            .overlay(alignment: .trailing) { Rectangle().fill(p.line).frame(width: 1) }
            ChatView.streaming(back: false).padding(.top, 12)
        }
        .foregroundStyle(p.fg)
    }
}

/// A Mac window frame for a still: the traffic lights over the content.
public struct MacChrome<Content: View>: View {
    @Environment(\.look) var look
    let content: Content
    public init(@ViewBuilder content: () -> Content) { self.content = content() }
    public var body: some View {
        ZStack(alignment: .topLeading) {
            content
            HStack(spacing: 8) {
                Circle().fill(Color.hex(0xff5f57)); Circle().fill(Color.hex(0xfebc2e)); Circle().fill(Color.hex(0x28c840))
            }
            .frame(width: 52, height: 12).padding(.leading, 14).padding(.top, 14)
        }
        .clipShape(.rect(cornerRadius: 12))
    }
}
