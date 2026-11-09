import CoreImage.CIFilterBuiltins
import SwiftUI

/// The iPhone's first run: welcome with the three ways in (Scan QR, Nearby, an address), then waiting for approval
/// with the check word shown, then paired, landing on Home. The Mac client's "Connect to a server" is the same three.
public enum PhoneStep: String, CaseIterable, Sendable { case welcome, scan, nearby, address, waiting, paired }

public struct PhoneOnboarding: View {
    @Environment(\.look) var look
    let step: PhoneStep
    public init(step: PhoneStep) { self.step = step }
    public var body: some View {
        let p = look.palette
        ZStack {
            Backdrop()
            VStack(spacing: 0) {
                switch step {
                case .welcome: welcome
                case .scan: scan
                case .nearby: titled("Nearby", "cooee servers on this Wi-Fi") { NearbyList() }
                case .address: titled("Enter address", "The address `cooee setup` printed") { AddressField() }
                case .waiting: waiting
                case .paired: paired
                }
            }
            .padding(.horizontal, 24).padding(.bottom, 30)
        }
        .foregroundStyle(p.fg)
    }

    var welcome: some View {
        let p = look.palette
        return VStack(spacing: 14) {
            Spacer()
            AppMark(size: 132)
            Text("cooee").font(.system(size: 40, weight: .bold, design: .rounded))
            Text("Your agents run on your Mac.\nTalk to them from here.").ui(17, .regular, look)
                .multilineTextAlignment(.center).foregroundStyle(p.mute)
            Spacer()
            WaysIn()
            Text("No server yet? Get cooee for Mac, or run `cooee setup` on a Mac or Linux box: both show a QR code.")
                .ui(13, .regular, look).foregroundStyle(p.mute).multilineTextAlignment(.center).padding(.top, 6)
        }
    }

    var scan: some View {
        let p = look.palette
        return VStack(spacing: 18) {
            StepHeader(title: "Scan QR", subtitle: "Point at the code on your Mac, or in the terminal")
            ZStack {
                RoundedRectangle(cornerRadius: 28).fill(Color.hex(0x1c1f1d))
                QRCode(text: "https://mini.tail4a2b.ts.net/pair?c=K7Q2MX", size: 150)
                    .padding(16).background(Color.white, in: .rect(cornerRadius: 10))
                    .rotationEffect(.degrees(-6)).shadow(radius: 12)
                ViewfinderCorners().stroke(p.l, style: StrokeStyle(lineWidth: 5, lineCap: .round)).frame(width: 240, height: 240)
            }
            .frame(height: 420)
            Label("Found mini · cooee 0.9", systemImage: "checkmark.circle.fill").ui(15, .semibold, look).foregroundStyle(p.g)
            Spacer()
            HStack(spacing: 10) {
                CardButton(title: "Nearby")
                CardButton(title: "Enter address")
            }
        }
    }

    var waiting: some View {
        let p = look.palette
        return VStack(spacing: 18) {
            Spacer()
            Image(systemName: "desktopcomputer.and.arrow.down").font(.system(size: 54)).foregroundStyle(p.link)
            Text("Approve on your Mac").ui(26, .bold, look)
            Text("mini asks you to approve this iPhone. Check that it shows the same word:").ui(16, .regular, look)
                .foregroundStyle(p.mute).multilineTextAlignment(.center)
            Text("maple").font(.system(size: 44, weight: .bold, design: .monospaced)).tracking(4).foregroundStyle(p.l)
                .padding(.horizontal, 28).padding(.vertical, 12).surface(look, radius: 22)
            HStack(spacing: 8) {
                Circle().fill(p.c).frame(width: 8, height: 8)
                Text("Waiting for approval · 4:51 left").ui(14, .regular, look).foregroundStyle(p.mute)
            }
            Text("Any paired device can approve it too: it got a notification.").ui(13, .regular, look)
                .foregroundStyle(p.mute).multilineTextAlignment(.center)
            Spacer()
            CardButton(title: "Cancel", tint: p.r)
        }
    }

    var paired: some View {
        let p = look.palette
        return VStack(spacing: 16) {
            Spacer()
            Image(systemName: "checkmark.circle.fill").font(.system(size: 84)).foregroundStyle(p.g)
            Text("Paired with mini").ui(28, .bold, look)
            Text("Guy approved this iPhone from the Mac.").ui(16, .regular, look).foregroundStyle(p.mute)
            HStack(spacing: -10) {
                ForEach(Fixtures.personas) { Avatar(persona: $0, size: 48).overlay(Circle().strokeBorder(p.bg, lineWidth: look.style == .modern ? 3 : 0)) }
            }
            .padding(.top, 10)
            Text("5 personas are waiting on Home").ui(14, .regular, look).foregroundStyle(p.mute)
            Spacer()
            CardButton(title: "Go to Home", primary: true)
        }
    }

    func titled<C: View>(_ t: String, _ s: String, @ViewBuilder _ c: () -> C) -> some View {
        VStack(spacing: 22) { StepHeader(title: t, subtitle: s); c(); Spacer() }
    }
}

/// The three ways to a server, shared by the iPhone welcome and the Mac's client mode.
struct WaysIn: View {
    @Environment(\.look) var look
    var body: some View {
        let p = look.palette
        VStack(spacing: 10) {
            way("qrcode.viewfinder", "Scan QR", "The code on your Mac, or in the terminal", primary: true)
            way("wifi", "Nearby", "2 servers on this Wi-Fi")
            way("keyboard", "Enter address", "mini.tail4a2b.ts.net")
        }
        .foregroundStyle(p.fg)
    }
    func way(_ s: String, _ t: String, _ sub: String, primary: Bool = false) -> some View {
        let p = look.palette
        return HStack(spacing: 14) {
            Image(systemName: s).font(.system(size: 20, weight: .semibold)).frame(width: 30)
                .foregroundStyle(primary ? p.l : p.link)
            VStack(alignment: .leading, spacing: 2) {
                Text(t).ui(17, .semibold, look)
                Text(sub).ui(13, .regular, look).foregroundStyle(p.mute)
            }
            Spacer()
            Image(systemName: "chevron.right").font(.system(size: 13, weight: .semibold)).foregroundStyle(p.mute)
        }
        .padding(.horizontal, 16).frame(height: 64)
        .surface(look, radius: 22, interactive: true)
    }
}

struct StepHeader: View {
    @Environment(\.look) var look
    let title: String, subtitle: String
    var body: some View {
        HStack(alignment: .top) {
            RoundButton(symbol: "chevron.left", ascii: "<", size: 40)
            Spacer()
        }
        .overlay {
            VStack(spacing: 4) {
                Text(title).ui(17, .semibold, look)
                Text((try? AttributedString(markdown: subtitle)) ?? AttributedString(subtitle)).ui(13, .regular, look)
                    .foregroundStyle(look.palette.mute)
            }
        }
        .padding(.top, 4)
    }
}

struct NearbyList: View {
    @Environment(\.look) var look
    var body: some View {
        let p = look.palette
        VStack(spacing: 10) {
            ForEach(Fixtures.nearby, id: \.0) { name, detail, icon in
                HStack(spacing: 14) {
                    Image(systemName: icon).font(.system(size: 24)).foregroundStyle(p.link).frame(width: 36)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(name).ui(17, .semibold, look)
                        Text(detail).ui(13, .regular, look).foregroundStyle(p.mute)
                    }
                    Spacer()
                    Text("Pair").ui(15, .semibold, look).foregroundStyle(p.link)
                }
                .padding(.horizontal, 16).frame(height: 68).surface(look, radius: 22, interactive: true)
            }
            HStack(spacing: 8) {
                ProgressDots()
                Text("Looking for more (Bonjour, _cooee._tcp)").ui(13, .regular, look).foregroundStyle(p.mute)
            }
            .padding(.top, 6)
            Text("Over Tailscale or far away? Scan the QR or enter its address.").ui(13, .regular, look)
                .foregroundStyle(p.mute).multilineTextAlignment(.center)
        }
    }
}

struct ProgressDots: View {
    @Environment(\.look) var look
    var body: some View {
        HStack(spacing: 4) { ForEach(0..<3) { i in Circle().fill(look.palette.mute.opacity(1 - Double(i) * 0.3)).frame(width: 6, height: 6) } }
    }
}

struct AddressField: View {
    @Environment(\.look) var look
    var body: some View {
        let p = look.palette
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                Text("mini.tail4a2b.ts.net").ui(17, .regular, look)
                Rectangle().fill(p.link).frame(width: 2, height: 22)
                Spacer()
            }
            .padding(.horizontal, 16).frame(height: 54).surface(look, radius: 18)
            Text("A name or IP, with :port if not 443. The server shows it under its QR code.").ui(13, .regular, look)
                .foregroundStyle(p.mute)
            CardButton(title: "Connect", primary: true)
        }
    }
}

/// The app's mark: the farm's horse in a glass disc.
struct AppMark: View {
    @Environment(\.look) var look
    var size: CGFloat = 120
    var body: some View {
        Sprite(name: "horse-ch-done").frame(width: size * 0.8, height: size * 0.6)
            .frame(width: size, height: size).surface(look, radius: size / 2)
    }
}

struct ViewfinderCorners: Shape {
    func path(in r: CGRect) -> Path {
        var p = Path(); let l: CGFloat = 36
        for (x, y, dx, dy) in [(r.minX, r.minY, 1.0, 1.0), (r.maxX, r.minY, -1.0, 1.0), (r.minX, r.maxY, 1.0, -1.0), (r.maxX, r.maxY, -1.0, -1.0)] {
            p.move(to: CGPoint(x: x, y: y + dy * l)); p.addLine(to: CGPoint(x: x, y: y)); p.addLine(to: CGPoint(x: x + dx * l, y: y))
        }
        return p
    }
}

/// A real QR code (CoreImage), crisp at any size.
struct QRCode: View {
    let text: String
    var size: CGFloat = 220
    var body: some View {
        if let cg = Self.image(text) {
            Image(decorative: cg, scale: 1).interpolation(.none).resizable().frame(width: size, height: size)
        } else {
            Color.gray.frame(width: size, height: size)
        }
    }
    static func image(_ text: String) -> CGImage? {
        let f = CIFilter.qrCodeGenerator()
        f.message = Data(text.utf8)
        f.correctionLevel = "M"
        guard let out = f.outputImage else { return nil }
        return CIContext().createCGImage(out, from: out.extent)
    }
}
