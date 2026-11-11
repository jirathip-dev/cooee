import CooeeCore
import SwiftUI

internal struct SceneBlock: View {
    @Environment(\.look) private var look
    internal let block: JSONValue

    internal var body: some View {
        if look.style == .scene {
            ZStack(alignment: .bottom) {
                Sprite(name: "scene-strip").scaledToFill().frame(height: 180).clipped()
                Scroller(axis: .horizontal) {
                    HStack(alignment: .bottom, spacing: 2) {
                        ForEach(SceneLane.placed(block.list("lanes"))) { lane in
                            ActButton(source: lane.source) { horse(lane) }
                                .accessibilityLabel("\(lane.name), \(lane.state.word)")
                        }
                    }
                    .padding(.horizontal, 8)
                }
                .frame(height: 140).padding(.bottom, 10)
            }
            .frame(height: 180).clipShape(.rect(cornerRadius: 16))
            .accessibilityElement(children: .contain).accessibilityLabel(block.string("label") ?? "")
        }
    }

    private func horse(_ lane: SceneLane) -> some View {
        let ink = Color.hex(0x29251a)
        let red = Palette.farmLight.red
        let needs = lane.state == .needs
        return VStack(spacing: 2) {
            Sprite(name: lane.sprite).frame(width: 92, height: 70)
            Text(lane.name).ui(12, .semibold, look).lineLimit(1).foregroundStyle(ink)
                .padding(.horizontal, 8).padding(.vertical, 3)
                .background(Color.hex(needs ? 0xf8dede : 0xf5efda).opacity(0.92), in: .capsule)
                .overlay(Capsule().strokeBorder(needs ? red : .clear, lineWidth: 1.5))
            Text(lane.state.word).ui(10, .medium, look)
                .foregroundStyle(needs || lane.state == .fail ? red : ink)
        }
        .frame(width: 96).contentShape(.rect)
    }
}
