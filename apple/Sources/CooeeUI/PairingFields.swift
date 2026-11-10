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
