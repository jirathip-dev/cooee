import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

extension MenuFeature.State {
    /// The picked colour scheme while the server still has it; nil: the theme's own.
    public var colours: ThemeScheme? { settings.colours.flatMap { looks.schemes[$0] } }

    /// Whether every screen draws dark: the picked scheme's mode, else the Look's scheme or the device's.
    public func dark(device: Bool) -> Bool {
        if let mode = colours?.mode { return mode == "dark" }
        switch settings.scheme {
        case .system: return device
        case .light: return false
        case .dark: return true
        }
    }

    /// Every screen's palette: the picked scheme's, else the theme's in this mode, else farm's.
    public func palette(device: Bool) -> Palette {
        let dark = dark(device: device)
        let tokens = colours?.tokens ?? looks.tokens(settings.theme, dark: dark)
        return tokens.flatMap { Palette(tokens: $0, dark: dark) } ?? .farm(dark ? .dark : .light)
    }
}

internal struct ColourPicks: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<MenuFeature>

    internal var body: some View {
        if !store.looks.schemes.isEmpty {
            Group2(header: "Colours") {
                row("Theme's own", nil, store.looks.tokens(store.settings.theme, dark: look.palette.dark), mode: nil)
                ForEach(store.looks.schemes.sorted { ($0.value.label, $0.key) < ($1.value.label, $1.key) }, id: \.key) {
                    row($1.label, $0, $1.tokens, mode: $1.mode)
                }
            }
        }
    }

    private func row(_ title: String, _ name: String?, _ tokens: [String: String]?, mode: String?) -> some View {
        let selected = store.settings.colours == name
        return Button {
            store.send(.colours(name))
        } label: {
            HStack(spacing: 12) {
                swatch(tokens ?? [:])
                Text(title).ui(16, .regular, look)
                Spacer()
                if let mode { Text(mode).ui(13, .regular, look).foregroundStyle(look.palette.mute) }
                if selected { Image(systemName: "checkmark").foregroundStyle(look.palette.link) }
            }
            .padding(.horizontal, 14).frame(height: 48).contentShape(.rect)
        }
        .buttonStyle(.plain).accessibilityAddTraits(selected ? .isSelected : [])
    }

    private func swatch(_ tokens: [String: String]) -> some View {
        let ink = { (key: String) in tokens[key].flatMap(Color.hex) ?? look.palette.mute }
        return HStack(spacing: 3) {
            ForEach(["--fg", "--l", "--g", "--a", "--r"], id: \.self) {
                Circle().fill(ink($0)).frame(width: 6, height: 6)
            }
        }
        .frame(width: 44, height: 26)
        .background(ink("--bg"), in: .rect(cornerRadius: 7))
        .overlay { RoundedRectangle(cornerRadius: 7).stroke(ink("--edge")) }
    }
}
