import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

private struct Spot: Equatable {
    fileprivate let name: String
    fileprivate let bounds: Anchor<CGRect>
}

private struct Spots: PreferenceKey {
    fileprivate static var defaultValue: [Spot] { [] }

    fileprivate static func reduce(value: inout [Spot], nextValue: () -> [Spot]) {
        value += nextValue()
    }
}

extension View {
    internal func reportable(_ name: String) -> some View {
        anchorPreference(key: Spots.self, value: .bounds) { [Spot(name: name, bounds: $0)] }
    }

    internal func reportable(block: JSONValue) -> some View {
        reportable(block.string("id") ?? block.type)
    }

    internal func reporting(_ store: StoreOf<AppFeature>) -> some View {
        overlayPreferenceValue(Spots.self) { spots in
            if let report = store.scope(state: \.report, action: \.report) {
                ZStack(alignment: .bottom) {
                    GeometryReader { proxy in
                        ReportTaps(store: report, frames: spots.map { ($0.name, proxy[$0.bounds]) })
                    }
                    ReportPanel(store: report)
                }
            }
        }
    }
}

private struct ReportTaps: View {
    @Environment(\.look) private var look
    fileprivate let store: StoreOf<ReportFeature>
    fileprivate let frames: [(name: String, frame: CGRect)]

    fileprivate var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear.contentShape(.rect).onTapGesture { point in
                let hits = frames.filter { $0.frame.contains(point) }
                if let hit = hits.min(by: { $0.frame.width * $0.frame.height < $1.frame.width * $1.frame.height }) {
                    store.send(.pick(hit.name))
                }
            }
            if let picked = frames.first(where: { $0.name == store.element })?.frame {
                RoundedRectangle(cornerRadius: 6).stroke(look.palette.link, lineWidth: 2)
                    .frame(width: picked.width, height: picked.height)
                    .offset(x: picked.minX, y: picked.minY)
                    .allowsHitTesting(false)
            }
        }
    }
}

private struct ReportPanel: View {
    @Environment(\.look) private var look
    @Bindable fileprivate var store: StoreOf<ReportFeature>

    fileprivate var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(store.element.map { "Report: \($0)" } ?? "Tap what is wrong").ui(13, .semibold, look)
            TextField("What is wrong?", text: $store.note, axis: .vertical).ui(15, .regular, look)
                .textFieldStyle(.plain)
            if let error = store.error { Text(error).ui(13, .regular, look).foregroundStyle(look.palette.red) }
            HStack {
                Button("Cancel") { store.send(.cancel) }.keyboardShortcut(.cancelAction)
                Spacer()
                Button("Send") { store.send(.send) }.disabled(store.report == nil || store.sending)
            }
            .ui(15, .semibold, look).foregroundStyle(look.palette.link)
        }
        .padding(14)
        .background(look.palette.card, in: .rect(cornerRadius: 14))
        .overlay(RoundedRectangle(cornerRadius: 14).stroke(look.palette.line))
        .padding(12)
    }
}
