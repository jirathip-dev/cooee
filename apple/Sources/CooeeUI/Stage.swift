import BlocksUI
import CooeeCore
import SwiftUI

internal struct Stage<Content: View>: View {
    @Environment(\.look) private var look
    internal let height: CGFloat
    internal var title: String?
    internal var night = false
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
            PackArt(art: art, path: "scene.svg", night: night).frame(height: height)
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
            TimelineView(.everyMinute) { context in
                let night = look.night ?? Weather.night(at: context.date)
                strip(stage, night: night)
                    .overlay {
                        if look.sign {
                            Sprite(name: "", pack: "sprites.svg#s-sign-\(SignLevel(scene: block).rawValue)")
                        }
                    }
                    .overlay { Sky(weather: look.sign ? .clear : Weather(load: block["load"]?.number), night: night) }
                    .overlay(alignment: .topTrailing) { SkyButtons(next: block["next"], night: night) }
            }
            .padding(.horizontal, -BlockScreen.inset)
            .accessibilityElement(children: .contain).accessibilityLabel(block.string("label") ?? "")
        }
    }

    private func strip(_ stage: CooeeCore.Stage, night: Bool) -> some View {
        Stage(height: 180, night: night) {
            Scroller(axis: .horizontal) {
                HStack(alignment: .bottom, spacing: 2) {
                    ForEach(SceneLane.placed(block.list("lanes"))) { lane in
                        ActButton(source: Source(lane.source)) { LaneFigure(lane: lane, stage: stage) }
                            .accessibilityLabel("\(lane.name), \(look.word(lane.state))")
                    }
                }
                .padding(.horizontal, 8)
            }
            .frame(height: 140).padding(.bottom, 10)
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
            Acting(motion: Motion(lane.state)) { pose in
                Sprite(name: sprite(pose.frame), pack: "sprites.svg#s-\(look.sign ? "scene-" : "")\(lane.state.pose)")
                    .frame(width: 92, height: 70).acting(pose)
            }
            Text(lane.name).ui(12, .semibold, look).lineLimit(1).foregroundStyle(ink)
                .padding(.horizontal, 8).padding(.vertical, 3)
                .background(stage.color(needs ? \.alertPaper : \.paper).opacity(0.92), in: .capsule)
                .overlay(Capsule().strokeBorder(needs ? alert : .clear, lineWidth: 1.5))
            Text(look.word(lane.state)).ui(10, .medium, look)
                .foregroundStyle(needs || lane.state == .fail ? alert : ink)
        }
        .frame(width: 96).contentShape(.rect)
    }

    private func sprite(_ frame: Int) -> String {
        lane.state == .working && look.moving ? "\(lane.sprite)-gait\(frame)" : lane.sprite
    }
}
