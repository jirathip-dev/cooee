import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

public struct MacConnect: View {
    @Environment(\.look) private var look
    private let store: StoreOf<PairingFeature>?
    private let setUp: () -> Void

    public var body: some View {
        let palette = look.palette
        ZStack {
            Backdrop()
            VStack(spacing: 18) {
                AppMark(size: 80)
                Text("Connect to a server").ui(26, .bold, look)
                Text("cooee runs on another Mac or a Linux box. This Mac only talks to it.").ui(14, .regular, look)
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
                Button(action: setUp) {
                    Text("Want this Mac to be the server? Set it up instead").ui(13, .regular, look)
                        .foregroundStyle(palette.link)
                }
                .buttonStyle(.plain)
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
            HStack(spacing: 8) {
                Image(systemName: "qrcode.viewfinder").foregroundStyle(palette.link)
                Text("Scan its QR with this Mac's camera").ui(14, .regular, look).foregroundStyle(palette.link)
            }
            .padding(.top, 8)
        }
    }

    public init() {
        store = nil
        setUp = {}
    }

    public init(store: StoreOf<PairingFeature>, setUp: @escaping () -> Void) {
        self.store = store
        self.setUp = setUp
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
