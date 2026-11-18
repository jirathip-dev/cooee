import CooeeCore
import SwiftUI

internal struct Stage<Content: View>: View {
    @Environment(\.look) private var look
    internal let height: CGFloat
    internal var title: String?
    @ViewBuilder internal let content: Content

    internal var body: some View {
        if look.stage != nil {
            ZStack(alignment: .bottom) {
                backdrop
                content
            }
            .frame(height: height)
        } else if let title, look.quiet {
            VStack(alignment: .leading, spacing: 8) {
                Text(look.label(.section, title)).ui(12, .regular, look).foregroundStyle(look.palette.mute)
                    .padding(.horizontal, 16)
                content
            }
        } else {
            content
        }
    }

    @ViewBuilder private var backdrop: some View {
        if let art = look.art {
            PackArt(art: art, path: "scene.svg").frame(height: height)
        } else {
            Sprite(name: "scene-strip").scaledToFill().frame(height: height).clipped()
        }
    }
}

extension CooeeCore.Stage {
    internal func color(_ token: KeyPath<Self, String>) -> Color { Color.hex(self[keyPath: token]) ?? .clear }
}

internal struct LaneStage: View {
    @Environment(\.look) private var look
    internal let block: JSONValue

    internal var body: some View {
        if let stage = look.stage {
            Stage(height: 180) {
                Scroller(axis: .horizontal) {
                    HStack(alignment: .bottom, spacing: 2) {
                        ForEach(SceneLane.placed(block.list("lanes"))) { lane in
                            ActButton(source: lane.source) { LaneFigure(lane: lane, stage: stage) }
                                .accessibilityLabel("\(lane.name), \(lane.state.word)")
                        }
                    }
                    .padding(.horizontal, 8)
                }
                .frame(height: 140).padding(.bottom, 10)
            }
            .padding(.horizontal, -BlocksView.inset)
            .accessibilityElement(children: .contain).accessibilityLabel(block.string("label") ?? "")
        }
    }
}

internal struct LaneFigure: View {
    @Environment(\.look) private var look
    internal let lane: SceneLane
    internal let stage: CooeeCore.Stage

    internal var body: some View {
        let (ink, alert) = (stage.color(\.ink), stage.color(\.alert))
        let needs = lane.state == .needs
        VStack(spacing: 2) {
            Sprite(name: lane.sprite).frame(width: 92, height: 70)
            Text(lane.name).ui(12, .semibold, look).lineLimit(1).foregroundStyle(ink)
                .padding(.horizontal, 8).padding(.vertical, 3)
                .background(stage.color(needs ? \.alertPaper : \.paper).opacity(0.92), in: .capsule)
                .overlay(Capsule().strokeBorder(needs ? alert : .clear, lineWidth: 1.5))
            Text(lane.state.word).ui(10, .medium, look)
                .foregroundStyle(needs || lane.state == .fail ? alert : ink)
        }
        .frame(width: 96).contentShape(.rect)
    }
}
