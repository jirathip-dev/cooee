import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct MacTopics: View {
    @Environment(\.look) private var look

    public var body: some View {
        let palette = look.palette
        MacChrome {
            HStack(spacing: 0) {
                VStack(spacing: 0) {
                    TopBar(title: HomeScreen.bundled.title, personas: Fixtures.personas, actions: HomeActions())
                        .padding(.top, 34)
                    Scroller {
                        PinnedRow(pins: Fixtures.personas.map(Pin.persona), rows: Fixtures.rows)
                            .scaleEffect(0.9, anchor: .leading).padding(.top, 4)
                        PersonaRows(selected: "cond").padding(.top, 8)
                    }
                }
                .frame(width: 360).background(palette.card.opacity(look.style == .modern ? 0.45 : 0.8))
                .overlay(alignment: .trailing) { Rectangle().fill(palette.line).frame(width: 1) }
                chat.padding(.top, 12)
            }
            .foregroundStyle(palette.foreground)
        }
        .background(Backdrop())
    }

    private var chat: some View {
        let topic = Story.topics[0]
        return VStack(spacing: 0) {
            ChatHeader(persona: Story.conductor, chips: [], back: false)
            HStack(spacing: 12) {
                TopicStrip(topics: Story.topics, current: .topic(topic.id))
                PluginRow(inline: true).fixedSize().padding(.trailing, 16).padding(.top, look.mono ? 0 : 6)
            }
            Scroller {
                VStack(alignment: .leading, spacing: 14) {
                    TopicStart(topic: topic)
                    ForEach(Story.items) { ItemView(item: $0) }
                }
                .frame(maxWidth: 760).padding(.horizontal, 16).padding(.top, 12)
            }
            .frame(maxHeight: .infinity)
            Composer(hint: "Message cooee queue").frame(maxWidth: 760).padding(.horizontal, 16).padding(.bottom, 20)
        }
    }

    public init() {}
}
