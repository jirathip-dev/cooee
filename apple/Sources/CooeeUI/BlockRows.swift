import CooeeCore
import SwiftUI

internal struct RelativeValue: View {
    internal let value: JSONValue

    internal var body: some View {
        TimelineView(.everyMinute) { context in Text(value.text(at: context.date) ?? "") }
    }
}

internal struct Figure: View {
    @Environment(\.look) private var look
    internal let figure: JSONValue

    internal var body: some View {
        let palette = look.palette
        if look.style == .scene, let coat = figure.string("coat") {
            let pose = LaneState(rawValue: figure.string("state") ?? "")?.pose ?? "idle"
            Sprite(name: "horse-\(coat)-\(pose)").frame(width: 37, height: 28)
        } else if let icon = figure.string("icon") {
            Image(systemName: icon).font(.system(size: 14, weight: .semibold)).foregroundStyle(palette.link).frame(
                width: 28)
        } else if let avatar = figure.string("avatar") {
            Text(avatar.prefix(2).uppercased()).ui(12, .bold, look).frame(width: 28, height: 28)
                .background(palette.edge.opacity(0.5), in: .circle)
        } else {
            Circle().fill(palette.tone(figure.string("state")) ?? palette.mute).frame(width: 10, height: 10).frame(
                width: 28)
        }
    }
}

internal struct RowButton: View {
    @Environment(\.look) private var look
    internal let button: JSONValue

    internal var body: some View {
        let tint = look.palette.tone(button.string("tone")) ?? look.palette.link
        ActButton(source: button) {
            Text(button.string("label") ?? "").ui(13, .semibold, look).foregroundStyle(tint)
                .padding(.horizontal, 12).frame(minHeight: 30)
                .background(look.mono ? Color.clear : tint.opacity(0.16), in: .capsule)
        }
    }
}

internal struct Lifted: ViewModifier {
    @Environment(\.look) private var look
    internal let lifted: Bool

    @ViewBuilder
    internal func body(content: Content) -> some View {
        if lifted {
            content.background(look.palette.background, in: .rect(cornerRadius: 12))
                .shadow(color: .black.opacity(0.4), radius: 12, y: 8).scaleEffect(1.02).offset(y: -6).zIndex(1)
        } else {
            content
        }
    }
}
