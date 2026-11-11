import SwiftUI

internal struct PairedDevice: Identifiable {
    internal let id: String
    internal let name: String
    internal let symbol: String
    internal let paired: String
    internal let seen: String
    internal var me = false
}

extension PairedDevice {
    internal static let fixtures: [PairedDevice] = [
        PairedDevice(
            id: "d1", name: "Guy's iPhone", symbol: "iphone.gen3", paired: "3 Nov", seen: "now", me: true),
        PairedDevice(id: "d2", name: "mini", symbol: "macmini", paired: "1 Nov", seen: "2 min ago"),
        PairedDevice(id: "d3", name: "iPad Air", symbol: "ipad", paired: "4 Nov", seen: "2 d ago"),
        PairedDevice(id: "d4", name: "Safari on air", symbol: "safari", paired: "28 Oct", seen: "9 d ago"),
    ]
}

public struct DevicesView: View {
    @Environment(\.look) private var look
    private let devices = PairedDevice.fixtures
    private let phone: Bool

    public var body: some View {
        SettingsPage(title: "Devices", phone: phone) {
            PairAskCard(label: "Pixel 9", word: "maple", ago: "1m ago")
            Group2(header: "Paired") {
                ForEach(devices) { device in
                    row(device, last: device.id == devices.last?.id)
                }
            }
            Group2 {
                SettingRow(symbol: "qrcode", title: "Pair a device", tint: look.palette.link)
                SettingRow(
                    symbol: "person.badge.key.fill", title: "Passkeys", tint: look.palette.purple, value: "2",
                    divider: false)
            }
        }
    }

    private func row(_ device: PairedDevice, last: Bool) -> some View {
        let palette = look.palette
        return Line(divider: !last, inset: look.mono ? 0 : 56) {
            HStack(spacing: 12) {
                if !look.mono {
                    Image(systemName: device.symbol).font(.system(size: 17)).foregroundStyle(palette.link)
                        .frame(width: 30)
                }
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 6) {
                        Text(device.name).ui(16, .medium, look)
                        if device.me { Pill(title: "This device", tint: palette.green) }
                    }
                    Text("paired \(device.paired) · seen \(device.seen)").ui(12, .regular, look)
                        .foregroundStyle(palette.mute)
                }
                Spacer(minLength: 6)
                Pill(title: device.me ? "Unpair" : "Remove", tint: palette.red)
            }
        }
    }

    public init(phone: Bool = true) {
        self.phone = phone
    }
}

internal struct PairAskCard: View {
    @Environment(\.look) private var look
    internal let label: String
    internal let word: String
    internal let ago: String

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text(look.mono ? "! pair a new device?" : "Pair a new device?").ui(17, .bold, look)
                Spacer()
                Text(ago).ui(13, .regular, look).foregroundStyle(palette.mute)
            }
            Text("\(label), over your tailnet").ui(14, .regular, look).foregroundStyle(palette.mute)
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                Text("check word").ui(13, .regular, look).foregroundStyle(palette.mute)
                Text(word).font(.system(size: 28, weight: .bold, design: .monospaced)).tracking(3)
                    .foregroundStyle(palette.lead)
            }
            HStack(spacing: 10) {
                CardButton(title: "Decline", tint: palette.red)
                CardButton(title: look.mono ? "approve · face id" : "Approve · Face ID", primary: true)
            }
        }
        .padding(16).surface(look, radius: 22)
    }
}
