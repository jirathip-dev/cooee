import BlocksUI
import CooeeCore
import SwiftUI

private struct PickNightKey: EnvironmentKey {
    fileprivate static var defaultValue: @MainActor (Bool) -> Void { { _ in } }
}

extension EnvironmentValues {
    internal var pickNight: @MainActor (Bool) -> Void {
        get { self[PickNightKey.self] }
        set { self[PickNightKey.self] = newValue }
    }
}

internal struct Sky: View {
    internal let weather: Weather
    internal let night: Bool

    internal var body: some View {
        ZStack(alignment: .topLeading) {
            if night { Color(red: 0.07, green: 0.1, blue: 0.2).opacity(0.5) }
            switch weather {
            case .clear: Color.clear
            case .cloudy: Color(white: 0.55).opacity(0.3)
            case .storm: Color(red: 0.11, green: 0.15, blue: 0.19).opacity(0.45)
            }
            HStack(spacing: 6) {
                if night { mark("moon.stars.fill") }
                if weather != .clear { mark(weather == .storm ? "cloud.bolt.rain.fill" : "cloud.fill") }
            }
            .padding(10)
        }
        .allowsHitTesting(false)
        .accessibilityElement().accessibilityLabel("\(weather.rawValue), \(night ? "night" : "day")")
    }

    private func mark(_ symbol: String) -> some View {
        Image(systemName: symbol).font(.system(size: 20, weight: .semibold)).foregroundStyle(.white.opacity(0.85))
    }
}

internal struct SkyButtons: View {
    @Environment(\.pickNight) private var pickNight
    internal let next: JSONValue?
    internal let night: Bool

    internal var body: some View {
        HStack(spacing: 6) {
            Button {
                pickNight(!night)
            } label: {
                RoundButton(symbol: "moon.fill", ascii: "☾", size: 32)
            }
            .buttonStyle(.plain).accessibilityLabel(night ? "Day" : "Night")
            if let next {
                ActButton(source: Source(next)) { RoundButton(symbol: "chevron.right", ascii: "›", size: 32) }
                    .accessibilityLabel("Next repo")
            }
        }
        .padding(8)
    }
}
