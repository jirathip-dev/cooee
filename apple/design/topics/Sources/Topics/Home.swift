import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct HomePerPersona: View {
    @Environment(\.look) private var look

    public var body: some View {
        ZStack(alignment: .top) {
            Backdrop()
            VStack(spacing: 0) {
                TopBar(title: HomeScreen.bundled.title, personas: Fixtures.personas, actions: HomeActions())
                Scroller {
                    PinnedRow(pins: Fixtures.personas.map(Pin.persona), rows: Fixtures.rows).padding(.top, 6)
                    PersonaRows().padding(.top, 14)
                }
            }
        }
        .foregroundStyle(look.palette.foreground)
    }

    public init() {}
}

internal struct PersonaRows: View {
    internal var selected: String?

    internal var body: some View {
        VStack(spacing: 0) {
            ChatRow(row: Fixtures.rows[0], persona: Story.coder, selected: selected == "coder")
            PersonaTopicsRow(persona: Story.conductor, topics: Story.topics, selected: selected == "cond")
            PersonaTopicsRow(persona: Story.researcher, topics: Story.research)
            ChatRow(row: Fixtures.rows[3], persona: Fixtures.persona("chief"))
            ChatRow(row: Fixtures.rows[4], persona: Fixtures.persona("am"))
        }
    }
}

internal struct PersonaTopicsRow: View {
    @Environment(\.look) private var look
    internal let persona: Persona
    internal let topics: [Topic]
    internal var selected = false

    private var opens: Topic { topics.first { $0.unread > 0 } ?? topics[0] }

    internal var body: some View {
        let palette = look.palette
        HStack(spacing: 12) {
            Avatar(persona: persona, size: 48)
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    Text(persona.name).ui(16, .semibold, look).lineLimit(1)
                    count
                    Spacer(minLength: 4)
                    Text(opens.ago).ui(13, .regular, look).foregroundStyle(opens.unread > 0 ? palette.link : palette.mute)
                }
                HStack(spacing: 6) {
                    Glyph(symbol: opens.folder == nil ? "bubble.left" : "folder", ascii: "/", size: 11)
                        .foregroundStyle(palette.mute)
                    Text("\(Text(opens.title).fontWeight(.semibold)) · \(opens.last)").ui(14, .regular, look)
                        .foregroundStyle(palette.mute).lineLimit(1)
                    Spacer(minLength: 4)
                    let unread = topics.map(\.unread).reduce(0, +)
                    if unread > 0 { Badge(count: unread) }
                }
            }
        }
        .padding(.horizontal, 16).padding(.vertical, 10)
        .background(selected ? palette.link.opacity(0.16) : .clear, in: .rect(cornerRadius: look.mono ? 0 : 14))
        .padding(.horizontal, selected ? 6 : 0)
    }

    private var count: some View {
        let palette = look.palette
        return Text(look.mono ? "[\(topics.count) topics]" : "\(topics.count) topics").ui(12, .medium, look)
            .foregroundStyle(palette.mute).padding(.horizontal, look.mono ? 0 : 7).frame(height: 20)
            .background(look.mono ? Color.clear : palette.edge.opacity(0.35), in: .capsule)
    }
}

public struct HomePerTopic: View {
    public var body: some View {
        HomeView(chats: Self.chats)
    }

    private static var chats: [ListedChat] {
        let conductor = Story.topics.map { topic in
            ListedChat(
                row: Row(
                    id: topic.id, persona: "cond", title: topic.title, last: topic.last, ago: topic.ago,
                    unseen: topic.unread, state: topic.working ? .working : .idle
                ),
                persona: Story.conductor
            )
        }
        let others = Fixtures.chats
        return [others[0], conductor[0], conductor[1], others[1], others[3], conductor[2], conductor[3], conductor[4]]
    }

    public init() {}
}
