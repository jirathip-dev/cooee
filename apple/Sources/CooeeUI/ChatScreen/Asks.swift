import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

/// What the chat's Asks chip and its one line show (#852), and how they open the sheet.
internal struct AsksBar {
    internal var count = 0
    internal var urgent = false
    internal var top = ""
    internal var open: () -> Void = {}
}

extension EnvironmentValues {
    @Entry internal var asks = AsksBar()
}

internal struct AsksPresenting: ViewModifier {
    @Bindable internal var store: StoreOf<ChatFeature>

    internal func body(content: Content) -> some View {
        let asks = store.asks
        let bar = AsksBar(count: asks.count, urgent: asks.urgent, top: asks.top?.text ?? "") {
            store.send(.area(.asks(.show(true))))
        }
        content
            .environment(\.asks, bar)
            .inspector(isPresented: Binding(get: { store.area.asks.open }, set: { store.send(.area(.asks(.show($0)))) })) {
                AsksSheet(asks: asks, answering: store.answering, done: { store.send(.area(.asks(.show(false)))) }) {
                    store.send(.cards(.answer($0, $1)))
                }
                .inspectorColumnWidth(420).presentationDetents([.large])
            }
    }
}

internal struct AsksSheet: View {
    @Environment(\.look) private var look
    internal let asks: Asks
    internal let answering: Set<Int>
    internal let done: () -> Void
    internal let answer: (Int, BlockAction) -> Void

    internal var body: some View {
        VStack(spacing: 0) {
            SheetBar(title: "Asks \(asks.count)", done: "Done", run: done)
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 12) {
                    ForEach(asks.sections) { section in
                        Caps(text: "\(section.group.label) · \(section.asks.count)")
                            .foregroundStyle(section.group.urgent ? look.palette.red : look.palette.mute)
                        ForEach(section.asks) { ask in
                            ItemView(item: .card(id: ask.id, poster: ask.plugin, block: ask.block))
                                .environment(\.act) { answer(ask.id, $0) }
                                .disabled(answering.contains(ask.id) || ask.settled)
                                .padding(.horizontal, 16)
                        }
                    }
                }
                .padding(.bottom, 24)
            }
        }
        .foregroundStyle(look.palette.foreground).background(look.palette.background)
    }
}

internal struct AsksChip: View {
    @Environment(\.look) private var look
    @Environment(\.asks) private var asks

    internal var body: some View {
        if asks.count > 0 {
            Button(action: asks.open) {
                HStack(spacing: 6) {
                    Text(look.label(.chip, "Asks")).ui(13, .semibold, look)
                    Badge(count: asks.count, tint: asks.urgent ? look.palette.red : look.palette.amber)
                }
                .padding(.horizontal, 12).frame(height: 30).chip(look)
            }
            .buttonStyle(.plain).accessibilityLabel("Asks \(asks.count)")
        }
    }
}

internal struct AsksLine: View {
    @Environment(\.look) private var look
    @Environment(\.asks) private var asks

    internal var body: some View {
        if asks.count > 0 {
            let tint = asks.urgent ? look.palette.red : look.palette.amber
            Button(action: asks.open) {
                HStack(spacing: 8) {
                    Text("\(asks.count) need you").fontWeight(.semibold)
                    Text("· \(asks.top)").foregroundStyle(look.palette.mute).lineLimit(1)
                    Spacer(minLength: 4)
                    Text(">").foregroundStyle(look.palette.mute)
                }
                .ui(14, .regular, look).padding(.horizontal, 14).frame(height: 40)
                .background(tint.opacity(0.12), in: look.pill)
                .overlay(look.pill.stroke(tint, lineWidth: 1))
            }
            .buttonStyle(.plain).accessibilityLabel("\(asks.count) need you")
        }
    }
}
