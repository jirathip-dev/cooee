import CooeeCore
import CooeeFeatures
import CoreImage.CIFilterBuiltins
import SwiftUI

internal struct NearbyRow: Sendable {
    internal let server: NearbyServer
    internal let icon: String
}

internal struct WaysIn: View {
    @Environment(\.look) private var look
    internal var show: (PhoneStep) -> Void = { _ in }
    internal var nearby: Int?

    private var scans: Bool {
        #if os(iOS)
            true
        #else
            false
        #endif
    }

    internal var body: some View {
        VStack(spacing: 10) {
            if scans || nearby == nil {
                way(.scan, "qrcode.viewfinder", "Scan QR", "The code on your Mac, or in the terminal", primary: true)
            }
            way(
                .nearby, "wifi", "Nearby",
                nearby.map { $0 == 0 ? "Servers on this Wi-Fi" : "\($0) on this Wi-Fi" } ?? "2 servers on this Wi-Fi"
            )
            way(.address, "keyboard", "Enter address", "server.example.com", primary: nearby != nil && !scans)
        }
        .foregroundStyle(look.palette.foreground)
    }

    private func way(_ step: PhoneStep, _ symbol: String, _ title: String, _ sub: String, primary: Bool = false)
        -> some View
    {
        let palette = look.palette
        return Button {
            show(step)
        } label: {
            HStack(spacing: 14) {
                Image(systemName: symbol).font(.system(size: 20, weight: .semibold)).frame(width: 30)
                    .foregroundStyle(primary ? palette.lead : palette.link)
                VStack(alignment: .leading, spacing: 2) {
                    Text(title).ui(17, .semibold, look)
                    Text(sub).ui(13, .regular, look).foregroundStyle(palette.mute)
                }
                Spacer()
                Image(systemName: "chevron.right").font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(palette.mute)
            }
            .padding(.horizontal, 16).frame(height: 64)
            .surface(look, radius: 22, interactive: true)
            .contentShape(.rect)
        }
        .buttonStyle(.plain)
        .accessibilityLabel(title).accessibilityHint(sub)
    }
}

private struct StepBackKey: EnvironmentKey {
    fileprivate static let defaultValue: @Sendable @MainActor () -> Void = {}
}

extension EnvironmentValues {
    internal var stepBack: @Sendable @MainActor () -> Void {
        get { self[StepBackKey.self] }
        set { self[StepBackKey.self] = newValue }
    }
}

internal struct StepHeader: View {
    @Environment(\.look) private var look
    @Environment(\.stepBack) private var back
    internal let title: String
    internal let subtitle: String

    internal var body: some View {
        HStack(alignment: .top) {
            Button {
                back()
            } label: {
                RoundButton(symbol: "chevron.left", ascii: "<", size: 40)
            }
            .buttonStyle(.plain).accessibilityLabel("Back")
            Spacer()
        }
        .overlay {
            VStack(spacing: 4) {
                Text(title).ui(17, .semibold, look)
                Text(BlockView.marked(subtitle)).ui(13, .regular, look).foregroundStyle(look.palette.mute)
            }
        }
        .padding(.top, 4)
    }
}

internal struct ProgressDots: View {
    @Environment(\.look) private var look

    internal var body: some View {
        HStack(spacing: 4) {
            ForEach(0..<3) { index in
                Circle().fill(look.palette.mute.opacity(1 - Double(index) * 0.3)).frame(width: 6, height: 6)
            }
        }
    }
}

internal struct AppMark: View {
    @Environment(\.look) private var look
    internal var size: CGFloat = 120

    internal var body: some View {
        Sprite(name: "horse-ch-done").frame(width: size * 0.8, height: size * 0.6)
            .frame(width: size, height: size).surface(look, radius: size / 2)
    }
}

internal struct ViewfinderCorners: Shape {
    internal func path(in rect: CGRect) -> Path {
        var path = Path()
        let arm: CGFloat = 36
        let corners = [
            (CGPoint(x: rect.minX, y: rect.minY), CGVector(dx: 1, dy: 1)),
            (CGPoint(x: rect.maxX, y: rect.minY), CGVector(dx: -1, dy: 1)),
            (CGPoint(x: rect.minX, y: rect.maxY), CGVector(dx: 1, dy: -1)),
            (CGPoint(x: rect.maxX, y: rect.maxY), CGVector(dx: -1, dy: -1)),
        ]
        for (corner, toward) in corners {
            path.move(to: CGPoint(x: corner.x, y: corner.y + toward.dy * arm))
            path.addLine(to: corner)
            path.addLine(to: CGPoint(x: corner.x + toward.dx * arm, y: corner.y))
        }
        return path
    }
}

internal struct QRCode: View {
    internal let text: String
    internal var size: CGFloat = 220

    internal var body: some View {
        if let image = Self.image(text) {
            Image(decorative: image, scale: 1).interpolation(.none).resizable().frame(width: size, height: size)
        } else {
            Color.gray.frame(width: size, height: size)
        }
    }

    internal static func image(_ text: String) -> CGImage? {
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(text.utf8)
        filter.correctionLevel = "M"
        guard let output = filter.outputImage else { return nil }
        return CIContext().createCGImage(output, from: output.extent)
    }
}
