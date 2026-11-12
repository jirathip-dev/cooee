import CooeeCore
import SwiftUI

internal struct OthersRow: View {
    @Environment(\.look) private var look
    internal let count: Int
    internal let open: () -> Void

    internal var body: some View {
        let palette = look.palette
        Button(action: open) {
            HStack(spacing: 12) {
                Image(systemName: "tray.full").font(.system(size: 20)).foregroundStyle(palette.mute)
                    .frame(width: 48, height: 48).modifier(RoundSurface(look: look))
                Text("Others").ui(16, .semibold, look)
                Spacer(minLength: 4)
                Text("\(count)").ui(12, .bold, look).foregroundStyle(palette.mute)
                    .padding(.horizontal, 8).frame(minHeight: 22).background(palette.line, in: .capsule)
            }
            .padding(.horizontal, 16).padding(.vertical, 10).contentShape(.rect)
        }
        .buttonStyle(.plain)
        .overlay(alignment: .top) { Rectangle().fill(palette.line).frame(height: 0.5) }
    }
}

internal struct OthersView: View {
    @Environment(\.look) private var look
    internal let chats: [ListedChat]
    internal let open: (String) -> Void
    internal let back: () -> Void

    internal var body: some View {
        let palette = look.palette
        ZStack(alignment: .topLeading) {
            Backdrop()
            VStack(alignment: .leading, spacing: 0) {
                BackLink(title: "Home", run: back)
                Scroller {
                    Text("Others").ui(34, .bold, look).padding(.horizontal, 16)
                    Text("Chats with no persona, and workflow whose source no installed plugin claims.")
                        .ui(14, .regular, look).foregroundStyle(palette.mute).padding(.horizontal, 16)
                    ForEach(Workflow.others(chats), id: \.name) { section in
                        Caps(text: section.name)
                        ChatList(chats: section.chats, title: nil, open: open)
                    }
                }
            }
        }
        .foregroundStyle(palette.foreground)
    }
}

internal struct BackLink: View {
    @Environment(\.look) private var look
    internal let title: String
    internal let run: () -> Void

    internal var body: some View {
        Button(action: run) {
            Text("‹ \(title)").ui(17, .regular, look).foregroundStyle(look.palette.link).frame(minHeight: 40)
        }
        .buttonStyle(.plain).accessibilityLabel("Back").padding(.horizontal, 14).padding(.vertical, 4)
    }
}

internal struct Caps: View {
    @Environment(\.look) private var look
    internal let text: String

    internal var body: some View {
        Text(text.uppercased()).ui(13, .semibold, look).foregroundStyle(look.palette.mute).kerning(0.6)
            .padding(.horizontal, 16).padding(.top, 18).padding(.bottom, 4)
    }
}

internal struct WorkflowSheet: View {
    @Environment(\.look) private var look
    internal let issues: [WorkflowIssue]
    internal let unfolded: String?
    internal let unfold: (String) -> Void
    internal let open: (String) -> Void
    internal let done: () -> Void

    internal var body: some View {
        let palette = look.palette
        VStack(spacing: 0) {
            SheetBar(title: "Workflow", done: "Done", run: done)
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    summary.padding(.horizontal, 16).padding(.top, 4)
                    ForEach(Workflow.groups(issues), id: \.name) { group in
                        Caps(text: group.name)
                        ForEach(group.issues) { issue in
                            Button {
                                unfold(issue.id)
                            } label: {
                                IssueRow(issue: issue, open: issue.id == unfolded)
                            }.buttonStyle(.plain)
                            if issue.id == unfolded { rounds(issue) }
                            Rectangle().fill(palette.line).frame(height: 0.5).padding(.leading, 64)
                        }
                    }
                }
            }
        }
        .foregroundStyle(palette.foreground).background(palette.background)
    }

    private var summary: some View {
        let source = issues.first?.latest.workflow?.source ?? "workflow"
        let needs = issues.count(where: \.needsYou)
        return HStack(spacing: 0) {
            Text("\(source) · \(issues.count) issue\(issues.count == 1 ? "" : "s")")
            if needs > 0 { Text(" · \(needs) needs you").fontWeight(.semibold).foregroundStyle(look.palette.red) }
        }
        .ui(14, .regular, look).foregroundStyle(look.palette.mute)
    }

    private func rounds(_ issue: WorkflowIssue) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(issue.rounds) { round in
                Button {
                    open(round.id)
                } label: {
                    HStack(spacing: 8) {
                        Circle().fill(look.palette.state(round.row.state)).frame(width: 10, height: 10)
                        Text(round.workflow?.stage ?? round.row.title).ui(15, .semibold, look)
                        Text(round.row.state.word).ui(14, .regular, look)
                            .foregroundStyle(look.palette.state(round.row.state))
                        Spacer()
                        Text("open chat ›").ui(14, .medium, look).foregroundStyle(look.palette.link)
                    }
                    .contentShape(.rect)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.leading, 64).padding(.trailing, 16).padding(.bottom, 12)
    }
}

internal struct IssueRow: View {
    @Environment(\.look) private var look
    internal let issue: WorkflowIssue
    internal let open: Bool

    internal var body: some View {
        let (palette, row) = (look.palette, issue.latest.row)
        let tint = palette.state(row.state)
        HStack(alignment: .center, spacing: 12) {
            Avatar(persona: issue.latest.persona, size: 40)
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 0) {
                    Text("\(issue.id) · ")
                    Text(issue.latest.workflow?.stage ?? "").fontWeight(.regular).foregroundStyle(tint)
                }
                .ui(16, .semibold, look).lineLimit(1)
                HStack(spacing: 0) {
                    Text(row.state.word).foregroundStyle(tint)
                    Text(" · \(row.title)").foregroundStyle(palette.mute)
                }
                .ui(14, .regular, look).lineLimit(1)
            }
            Spacer(minLength: 4)
            Text("\(row.ago) \(open ? "⌄" : "›")").ui(14, .regular, look).foregroundStyle(palette.mute)
        }
        .padding(.horizontal, 16).padding(.vertical, 10).contentShape(.rect)
    }
}

internal struct NeutralAvatar: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal let size: CGFloat

    internal var body: some View {
        Circle().fill(look.palette.mute.opacity(0.22)).frame(width: size, height: size).overlay {
            if persona.look.coat.isEmpty {
                Text(persona.look.initials).ui(size * 0.34, .bold, look)
            } else {
                Sprite(name: "horse-\(persona.look.coat)-\(persona.pose)").frame(width: size * 0.62)
            }
        }
    }
}

extension Palette {
    internal func state(_ state: LaneState) -> Color {
        switch state {
        case .needs, .fail: red
        case .working: amber
        case .done, .merged: green
        case .idle: mute
        }
    }
}

internal struct WorkflowChip: View {
    @Environment(\.look) private var look
    internal let count: Int
    internal let lit: Bool
    internal let open: () -> Void

    internal var body: some View {
        Button(action: open) {
            HStack(spacing: 6) {
                Text(look.mono ? "[workflow]" : "Workflow").ui(13, .semibold, look).foregroundStyle(look.palette.green)
                if count > 0 { Badge(count: count) }
            }
            .padding(.horizontal, 12).frame(height: 30).modifier(RoundSurface(look: look))
            .overlay(Capsule().strokeBorder(look.palette.link, lineWidth: lit ? 1.5 : 0))
        }
        .buttonStyle(.plain).accessibilityLabel("Workflow")
    }
}
