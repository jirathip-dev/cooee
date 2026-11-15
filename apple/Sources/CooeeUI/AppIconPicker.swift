import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct IconImage: View {
    internal let icon: AppIcon
    internal let size: CGFloat

    internal var body: some View {
        Image("\(icon.asset)Preview").resizable().frame(width: size, height: size)
            .clipShape(.rect(cornerRadius: size * 0.224, style: .continuous))
    }
}

internal struct AppIconPicker: View {
    @Environment(\.look) private var look
    internal let store: StoreOf<MenuFeature>

    internal var body: some View {
        let settings = store.settings
        let followed = AppIcon.following(settings.style)
        Group2 {
            Button {
                store.send(.icon(nil))
            } label: {
                HStack(spacing: 12) {
                    IconImage(icon: followed, size: 44)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Follow the look").ui(16, .regular, look)
                        Text("\(settings.icon == nil ? "Now:" : "Off · would show") \(label(followed))")
                            .ui(13, .regular, look).foregroundStyle(look.palette.mute).lineLimit(1)
                    }
                    Spacer()
                    if settings.icon == nil { Image(systemName: "checkmark").foregroundStyle(look.palette.link) }
                }
                .padding(.horizontal, 14).padding(.vertical, 10).contentShape(.rect)
            }
            .buttonStyle(.plain).accessibilityLabel("Follow the look")
            .accessibilityAddTraits(settings.icon == nil ? .isSelected : [])
        }
        ForEach(IconSet.allCases, id: \.self) { set in
            Group2(header: set.rawValue.capitalized) {
                HStack(alignment: .top, spacing: 4) { ForEach(set.icons, id: \.self) { tile($0, settings) } }
                    .padding(.horizontal, 8).padding(.top, 12).padding(.bottom, 10)
            }
        }
        if let error = store.error { Text(error).ui(13, .regular, look).foregroundStyle(look.palette.red) }
    }

    private func label(_ icon: AppIcon) -> String { "\(icon.name) · \(icon.set.rawValue.capitalized)" }

    private func tile(_ icon: AppIcon, _ settings: CooeeCore.Settings) -> some View {
        let picked = settings.icon == icon
        let now = settings.icon == nil && settings.appIcon == icon
        return Button {
            store.send(.icon(icon))
        } label: {
            VStack(spacing: 6) {
                IconImage(icon: icon, size: 64).padding(3)
                    .overlay {
                        RoundedRectangle(cornerRadius: 17).strokeBorder(
                            picked ? look.palette.link : now ? look.palette.mute : .clear,
                            style: StrokeStyle(lineWidth: picked ? 2.5 : 2, dash: now ? [5, 3] : [])
                        )
                    }
                    .overlay(alignment: .bottomTrailing) { if picked { badge } }
                Text(icon.name).ui(12, picked ? .semibold : .regular, look).lineLimit(1)
                    .foregroundStyle(picked ? look.palette.link : look.palette.foreground)
            }
            .frame(maxWidth: .infinity).contentShape(.rect)
        }
        .buttonStyle(.plain).accessibilityLabel(icon.name).accessibilityAddTraits(picked ? .isSelected : [])
    }

    private var badge: some View {
        Image(systemName: "checkmark").font(.system(size: 11, weight: .bold)).foregroundStyle(look.palette.background)
            .frame(width: 22, height: 22).background(look.palette.link, in: .circle)
            .overlay { Circle().strokeBorder(look.palette.background, lineWidth: 2) }.offset(x: 7, y: 7)
    }
}
