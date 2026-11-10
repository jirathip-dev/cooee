import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

public struct PhoneOnboarding: View {
    @Environment(\.look) private var look
    private let fixed: PhoneStep
    private let store: StoreOf<PairingFeature>?

    private var step: PhoneStep { store?.step ?? fixed }

    public var body: some View {
        ZStack {
            Backdrop()
            VStack(spacing: 0) { screen }
                .padding(.horizontal, 24).padding(.bottom, 30)
        }
        .foregroundStyle(look.palette.foreground)
        .environment(\.stepBack, { show(.welcome) })
    }

    @ViewBuilder private var screen: some View {
        switch step {
        case .welcome:
            Welcome(store: store, show: show)
        case .scan:
            ScanStep(store: store, show: show)
        case .nearby:
            titled("Nearby", "cooee servers on this Wi-Fi") { NearbyList(store: store, connect: connect) }
        case .address:
            titled("Enter address", "The address `cooee setup` printed") {
                AddressField(store: store, connect: connect)
            }
        case .waiting:
            Waiting(store: store)
        case .paired:
            Paired(store: store)
        }
    }

    public init(step: PhoneStep) {
        fixed = step
        store = nil
    }

    public init(store: StoreOf<PairingFeature>) {
        fixed = .welcome
        self.store = store
    }

    private func show(_ step: PhoneStep) { store?.send(.show(step)) }

    private func connect(_ typed: String) { store?.send(.connect(typed)) }

    private func titled(_ title: String, _ subtitle: String, @ViewBuilder _ content: () -> some View) -> some View {
        VStack(spacing: 22) {
            StepHeader(title: title, subtitle: subtitle)
            content()
            Spacer()
        }
    }
}

internal struct Welcome: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<PairingFeature>?
    internal let show: (PhoneStep) -> Void

    internal var body: some View {
        let palette = look.palette
        VStack(spacing: 14) {
            Spacer()
            AppMark(size: 132)
            Text("cooee").font(.system(size: 40, weight: .bold, design: .rounded))
            Text("Your agents run on your Mac.\nTalk to them from here.").ui(17, .regular, look)
                .multilineTextAlignment(.center).foregroundStyle(palette.mute)
            Spacer()
            WaysIn(show: show, nearby: store.map { $0.nearby.count })
            Text("No server yet? Get cooee for Mac, or run `cooee setup` on a Mac or Linux box: both show a QR code.")
                .ui(13, .regular, look).foregroundStyle(palette.mute).multilineTextAlignment(.center).padding(.top, 6)
        }
    }
}

internal struct ScanStep: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<PairingFeature>?
    internal let show: (PhoneStep) -> Void

    internal var body: some View {
        let palette = look.palette
        VStack(spacing: 18) {
            StepHeader(title: "Scan QR", subtitle: "Point at the code on your Mac, or in the terminal")
            viewfinder.frame(height: 420)
            if let store {
                if let error = store.error { Text(error).ui(15, .semibold, look).foregroundStyle(palette.red) }
            } else {
                Label("Found mini · cooee 0.9", systemImage: "checkmark.circle.fill").ui(15, .semibold, look)
                    .foregroundStyle(palette.green)
            }
            Spacer()
            HStack(spacing: 10) {
                Button {
                    show(.nearby)
                } label: {
                    CardButton(title: "Nearby")
                }.buttonStyle(.plain)
                Button {
                    show(.address)
                } label: {
                    CardButton(title: "Enter address")
                }.buttonStyle(.plain)
            }
        }
    }

    private var viewfinder: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 28).fill(Color.hex(0x1c1f1d))
            #if os(iOS)
                if let store { QRScanner { store.send(.connect($0)) }.clipShape(.rect(cornerRadius: 28)) }
            #endif
            if store == nil {
                QRCode(text: "https://server.example.com/pair?c=K7Q2MX", size: 150)
                    .padding(16).background(Color.white, in: .rect(cornerRadius: 10))
                    .rotationEffect(.degrees(-6)).shadow(radius: 12)
            }
            ViewfinderCorners().stroke(look.palette.lead, style: StrokeStyle(lineWidth: 5, lineCap: .round))
                .frame(width: 240, height: 240)
        }
    }
}

internal struct Waiting: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<PairingFeature>?

    internal var body: some View {
        let palette = look.palette
        VStack(spacing: 18) {
            Spacer()
            Image(systemName: "desktopcomputer.and.arrow.down").font(.system(size: 54)).foregroundStyle(palette.link)
            Text("Approve on your Mac").ui(26, .bold, look)
            Text("\(store?.host ?? "mini") asks you to approve this iPhone. Check that it shows the same word:")
                .ui(16, .regular, look).foregroundStyle(palette.mute).multilineTextAlignment(.center)
            Text(store?.word ?? "maple").font(.system(size: 44, weight: .bold, design: .monospaced)).tracking(4)
                .foregroundStyle(palette.lead)
                .padding(.horizontal, 28).padding(.vertical, 12).surface(look, radius: 22)
            HStack(spacing: 8) {
                Circle().fill(palette.cyan).frame(width: 8, height: 8)
                countdown
            }
            Text("Any paired device can approve it too: it got a notification.").ui(13, .regular, look)
                .foregroundStyle(palette.mute).multilineTextAlignment(.center)
            Spacer()
            Button {
                store?.send(.cancel)
            } label: {
                CardButton(title: "Cancel", tint: palette.red)
            }.buttonStyle(.plain)
        }
    }

    @ViewBuilder private var countdown: some View {
        if let until = store?.until {
            TimelineView(.periodic(from: .now, by: 1)) { context in
                let left = max(0, Int(until.timeIntervalSince(context.date)))
                Text("Waiting for approval · \(left / 60):\(String(format: "%02d", left % 60)) left")
                    .ui(14, .regular, look).foregroundStyle(look.palette.mute)
            }
        } else {
            Text("Waiting for approval · 4:51 left").ui(14, .regular, look).foregroundStyle(look.palette.mute)
        }
    }
}

internal struct Paired: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<PairingFeature>?

    internal var body: some View {
        let palette = look.palette
        let personas = store.map { $0.personas.map { Persona($0) } } ?? Fixtures.personas
        VStack(spacing: 16) {
            Spacer()
            Image(systemName: "checkmark.circle.fill").font(.system(size: 84)).foregroundStyle(palette.green)
            Text("Paired with \(store?.host ?? "mini")").ui(28, .bold, look)
            if store == nil {
                Text("Guy approved this iPhone from the Mac.").ui(16, .regular, look).foregroundStyle(palette.mute)
            }
            HStack(spacing: -10) {
                ForEach(personas) { persona in
                    Avatar(persona: persona, size: 48)
                        .overlay(Circle().strokeBorder(palette.background, lineWidth: look.style == .modern ? 3 : 0))
                }
            }
            .padding(.top, 10)
            Text("\(personas.count) personas are waiting on Home").ui(14, .regular, look).foregroundStyle(palette.mute)
            Spacer()
            Button {
                store?.send(.home)
            } label: {
                CardButton(title: "Go to Home", primary: true)
            }.buttonStyle(.plain)
        }
    }
}
