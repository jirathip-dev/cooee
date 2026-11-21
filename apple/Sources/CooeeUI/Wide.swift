import ComposableArchitecture
import CooeeFeatures
import SwiftUI

public struct MacConnect: View {
    @Environment(\.look) private var look
    private let store: StoreOf<PairingFeature>?
    @State private var scanning = false

    public var body: some View {
        let palette = look.palette
        ZStack {
            Backdrop()
            VStack(spacing: 18) {
                AppMark(size: 80)
                Text("Connect to a server").ui(26, .bold, look)
                Text("cooee runs on this Mac (`cooee setup`), another Mac or a Linux box.").ui(14, .regular, look)
                    .foregroundStyle(palette.mute)
                HStack(alignment: .top, spacing: 18) {
                    VStack(alignment: .leading, spacing: 10) {
                        Text("NEARBY").ui(11, .semibold, look).foregroundStyle(palette.mute)
                        NearbyList(store: store) { store?.send(.connect($0)) }
                    }
                    address
                }
                .frame(maxWidth: 760)
                Spacer()
            }
            .padding(36)
        }
        .foregroundStyle(palette.foreground)
    }

    private var address: some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 10) {
            Text("ADDRESS").ui(11, .semibold, look).foregroundStyle(palette.mute)
            AddressField(store: store) { store?.send(.connect($0)) }
            Button {
                scanning.toggle()
            } label: {
                HStack(spacing: 8) {
                    Image(systemName: "qrcode.viewfinder")
                    Text("Scan its QR with this Mac's camera").ui(14, .regular, look)
                }
                .foregroundStyle(palette.link)
            }
            .buttonStyle(.plain).padding(.top, 8)
            #if os(macOS)
                if scanning, let store {
                    MacQRScanner { store.send(.connect($0)) }.frame(height: 200).clipShape(.rect(cornerRadius: 12))
                }
            #endif
        }
    }

    public init(store: StoreOf<PairingFeature>? = nil) {
        self.store = store
    }
}

public struct MacChrome<Content: View>: View {
    private let content: Content

    public var body: some View {
        ZStack(alignment: .topLeading) {
            content
            HStack(spacing: 8) {
                Circle().fill(Color.hex(0xff5f57))
                Circle().fill(Color.hex(0xfebc2e))
                Circle().fill(Color.hex(0x28c840))
            }
            .frame(width: 52, height: 12).padding(.leading, 14).padding(.top, 14)
        }
        .clipShape(.rect(cornerRadius: 12))
    }

    public init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }
}
