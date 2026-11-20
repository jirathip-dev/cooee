import CooeeCore
import SwiftUI

internal struct SkillRows: View {
    @Environment(\.look) private var look
    internal let screen: JSONValue
    internal let add: (String) -> Void

    internal var body: some View {
        let block = screen.list("blocks").first { $0.type == "skills" }
        let (mine, library) = (block?.list("rows") ?? [], block?.list("library") ?? [])
        if !mine.isEmpty || !library.isEmpty {
            Group2(header: "Skills · \(mine.count) of 20") {
                ForEach(Array((mine + library).enumerated()), id: \.offset) { index, skill in
                    row(skill, adding: index >= mine.count)
                }
            }
        }
    }

    private func row(_ skill: JSONValue, adding: Bool) -> some View {
        let (palette, name) = (look.palette, skill.string("name") ?? "")
        return HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(name).ui(15, .medium, look)
                Text(skill.string("description") ?? "").ui(12, .regular, look).foregroundStyle(palette.mute)
                    .lineLimit(2)
            }
            Spacer(minLength: 4)
            if adding {
                Button {
                    add(name)
                } label: {
                    Text(look.label(.chip, "+ Add")).ui(14, .semibold, look).foregroundStyle(palette.link)
                }
                .buttonStyle(.plain).accessibilityLabel("Add skill \(name)")
            }
        }
        .padding(.horizontal, 14).padding(.vertical, 9)
        .overlay(alignment: .bottom) { Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, 14) }
    }
}
