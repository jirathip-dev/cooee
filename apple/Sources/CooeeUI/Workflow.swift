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
        ZStack(alignment: .topLeading) {
            Backdrop()
            VStack(alignment: .leading, spacing: 0) {
                Button(action: back) { RoundButton(symbol: "chevron.left", ascii: "<", size: 40) }
                    .buttonStyle(.plain).accessibilityLabel("Back").padding(.horizontal, 14).padding(.vertical, 8)
                Scroller {
                    ChatList(chats: chats, title: "Others", open: open)
                    Text("Chats with no persona, and workflow sessions no attached plugin claims.")
                        .ui(13, .regular, look).foregroundStyle(look.palette.mute).padding(16)
                }
            }
        }
        .foregroundStyle(look.palette.foreground)
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
        VStack(spacing: 0) {
            SheetBar(title: "Workflow", done: "Done", run: done)
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(issues) { issue in
                        Button {
                            unfold(issue.id)
                        } label: {
                            IssueRow(issue: issue)
                        }.buttonStyle(.plain)
                        if issue.id == unfolded { rounds(issue) }
                        Rectangle().fill(look.palette.line).frame(height: 0.5)
                    }
                    if issues.isEmpty {
                        Text("No workflow sessions.").foregroundStyle(look.palette.mute).padding(16)
                    }
                }
            }
        }
        .foregroundStyle(look.palette.foreground).background(look.palette.background)
    }

    private func rounds(_ issue: WorkflowIssue) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            ForEach(issue.rounds) { round in
                Button {
                    open(round.id)
                } label: {
                    HStack(spacing: 8) {
                        Text(round.row.state.figure).ui(13, .bold, look)
                        Text(round.workflow?.stage ?? round.row.title).ui(14, .semibold, look)
                        Text(round.row.state.word).ui(13, .regular, look).foregroundStyle(look.palette.mute)
                        Spacer()
                        Text("open chat ›").ui(13, .medium, look).foregroundStyle(look.palette.link)
                    }
                    .contentShape(.rect)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.leading, 44).padding(.trailing, 16).padding(.bottom, 10)
    }
}

internal struct IssueRow: View {
    @Environment(\.look) private var look
    internal let issue: WorkflowIssue

    internal var body: some View {
        let (palette, row) = (look.palette, issue.latest.row)
        HStack(alignment: .top, spacing: 12) {
            Text(row.state.figure).ui(13, .bold, look).frame(width: 32, alignment: .leading)
                .foregroundStyle(issue.needsYou ? palette.red : palette.cyan)
            VStack(alignment: .leading, spacing: 3) {
                Text("\(issue.id) · \(issue.latest.workflow?.stage ?? "")").ui(15, .semibold, look).lineLimit(1)
                Text("\(row.state.word) · \(row.last)").ui(13, .regular, look).foregroundStyle(palette.mute)
                    .lineLimit(1)
            }
            Spacer(minLength: 4)
            Text(row.ago).ui(13, .regular, look).foregroundStyle(palette.mute)
        }
        .padding(.horizontal, 16).padding(.vertical, 10).contentShape(.rect)
    }
}

internal struct WorkflowChip: View {
    @Environment(\.look) private var look
    internal let count: Int
    internal let open: () -> Void

    internal var body: some View {
        Button(action: open) {
            HStack(spacing: 6) {
                Text(look.mono ? "[workflow]" : "Workflow").ui(13, .semibold, look).foregroundStyle(look.palette.green)
                if count > 0 { Badge(count: count) }
            }
            .padding(.horizontal, 12).frame(height: 30).modifier(RoundSurface(look: look))
        }
        .buttonStyle(.plain).accessibilityLabel("Workflow")
    }
}
