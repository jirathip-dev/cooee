import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct NearbyList: View {
    @Environment(\.look) private var look
    internal var store: StoreOf<PairingFeature>?
    internal var connect: (String) -> Void = { _ in }

    private var found: [NearbyRow] {
        store.map { $0.nearby.map { NearbyRow(server: $0, icon: "desktopcomputer") } } ?? Fixtures.nearby
    }

    internal var body: some View {
        let palette = look.palette
        VStack(spacing: 10) {
            ForEach(found, id: \.server.name) { row in server(row) }
            if let error = store?.error { Text(error).ui(14, .semibold, look).foregroundStyle(palette.red) }
            HStack(spacing: 8) {
                ProgressDots()
                Text("Looking for more (Bonjour, _cooee._tcp)").ui(13, .regular, look).foregroundStyle(palette.mute)
            }
            .padding(.top, 6)
            Text("Over Tailscale or far away? Scan the QR or enter its address.").ui(13, .regular, look)
                .foregroundStyle(palette.mute).multilineTextAlignment(.center)
        }
        .onAppear { store?.send(.browse) }
    }

    private func server(_ row: NearbyRow) -> some View {
        let palette = look.palette
        return Button {
            connect(row.server.address)
        } label: {
            HStack(spacing: 14) {
                Image(systemName: row.icon).font(.system(size: 24)).foregroundStyle(palette.link).frame(width: 36)
                VStack(alignment: .leading, spacing: 2) {
                    Text(row.server.name).ui(17, .semibold, look)
                    Text(row.server.address).ui(13, .regular, look).foregroundStyle(palette.mute)
                }
                Spacer()
                Text("Pair").ui(15, .semibold, look).foregroundStyle(palette.link)
            }
            .padding(.horizontal, 16).frame(height: 68).surface(look, radius: 22, interactive: true)
            .contentShape(.rect)
        }
        .buttonStyle(.plain)
    }
}

internal struct AddressField: View {
    @Environment(\.look) private var look
    internal var store: StoreOf<PairingFeature>?
    internal var connect: (String) -> Void = { _ in }

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                field
                Spacer()
            }
            .padding(.horizontal, 16).frame(height: 54).surface(look, radius: 18)
            Text("A name or IP, with :port if not 443. The server shows it under its QR code.").ui(13, .regular, look)
                .foregroundStyle(palette.mute)
            if let error = store?.error { Text(error).ui(14, .semibold, look).foregroundStyle(palette.red) }
            Button {
                connect(store?.address ?? "")
            } label: {
                CardButton(title: "Connect", primary: true)
            }
            .buttonStyle(.plain)
        }
    }

    @ViewBuilder private var field: some View {
        if let store {
            TextField(
                "server.example.com",
                text: Binding(get: { store.address }, set: { store.send(.addressChanged($0)) })
            )
            .ui(17, .regular, look)
            .textFieldStyle(.plain).autocorrectionDisabled().onSubmit { connect(store.address) }
            #if os(iOS)
                .textInputAutocapitalization(.never).keyboardType(.URL)
            #endif
        } else {
            Text("server.example.com").ui(17, .regular, look)
            Rectangle().fill(look.palette.link).frame(width: 2, height: 22)
        }
    }
}

internal struct PairingPart: View {
    @Environment(\.look) private var look
    internal let block: JSONValue
    internal let store: StoreOf<PairingFeature>?

    internal var body: some View {
        switch block.type {
        case "scanner": Viewfinder(store: store).frame(height: block["height"]?.number.map { CGFloat($0) })
        case "ways": WaysIn(show: show, nearby: store.map { $0.nearby.count })
        case "nearby": NearbyList(store: store) { store?.send(.connect($0)) }
        case "address": AddressField(store: store) { store?.send(.connect($0)) }
        case "countdown": Countdown(text: block.string("text") ?? "", until: store?.until)
        case "avatars": Avatars(personas: store.map { $0.personas.map { Persona($0) } } ?? Fixtures.personas)
        case "buttons": buttons
        default: StepHeader(title: block.string("title") ?? "", subtitle: block.string("subtitle") ?? "")
        }
    }

    private var buttons: some View {
        HStack(spacing: 10) {
            ForEach(block.list("rows"), id: \.self) { row in
                Button {
                    tap(row)
                } label: {
                    CardButton(
                        title: row.string("title") ?? "", primary: row["primary"]?.bool == true,
                        tint: look.palette.tone(row.string("tone")))
                }
                .buttonStyle(.plain)
            }
        }
    }

    private func show(_ step: PhoneStep) { store?.send(.show(step)) }

    private func tap(_ row: JSONValue) {
        if let step = row.string("step").flatMap(PhoneStep.init(rawValue:)) {
            show(step)
        } else if row.string("action") == "cancel" {
            store?.send(.cancel)
        } else if row.string("action") == "home" {
            store?.send(.home)
        }
    }
}

private struct Viewfinder: View {
    @Environment(\.look) private var look
    fileprivate let store: StoreOf<PairingFeature>?

    fileprivate var body: some View {
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

private struct Countdown: View {
    @Environment(\.look) private var look
    fileprivate let text: String
    fileprivate let until: Date?

    fileprivate var body: some View {
        HStack(spacing: 8) {
            Circle().fill(look.palette.cyan).frame(width: 8, height: 8)
            if let until {
                TimelineView(.periodic(from: .now, by: 1)) { context in
                    let left = max(0, Int(until.timeIntervalSince(context.date)))
                    line(
                        text.replacingOccurrences(
                            of: "{left}", with: "\(left / 60):\(String(format: "%02d", left % 60))"))
                }
            } else {
                line(text)
            }
        }
    }

    private func line(_ text: String) -> some View {
        Text(text).ui(14, .regular, look).foregroundStyle(look.palette.mute)
    }
}

private struct Avatars: View {
    @Environment(\.look) private var look
    fileprivate let personas: [Persona]

    fileprivate var body: some View {
        HStack(spacing: -10) {
            ForEach(personas) { persona in
                Avatar(persona: persona, size: 48)
                    .overlay(Circle().strokeBorder(look.palette.background, lineWidth: look.style == .modern ? 3 : 0))
            }
        }
    }
}
