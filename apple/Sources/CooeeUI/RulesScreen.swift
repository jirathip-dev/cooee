import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct BackBar: View {
    @Environment(\.look) private var look
    internal let back: String
    internal let title: String
    internal var run: () -> Void = {}

    internal var body: some View {
        HStack {
            Button(action: run) {
                Text(look.label(.back, back)).ui(16, .regular, look).foregroundStyle(look.palette.link)
            }
            .buttonStyle(.plain)
            Spacer()
        }
        .overlay { Text(look.label(.section, title)).ui(17, .semibold, look) }
        .padding(.horizontal, 16).frame(height: 44)
    }
}

internal struct RulesPage: View {
    @Environment(\.look) private var look
    internal let name: String
    internal let rules: [Rule]
    internal var agent: AgentOption?
    internal var fresh: String?
    internal var swiped: String?
    internal var back: () -> Void = {}
    internal var add: () -> Void = {}
    internal var remove: (Rule) -> Void = { _ in }

    internal var body: some View {
        VStack(spacing: 0) {
            BackBar(back: name, title: "Rules", run: back)
            Scroller {
                RulesBody(
                    name: name, rules: rules, agent: agent, fresh: fresh, swiped: swiped, add: add, remove: remove
                )
                .padding(.horizontal, 16).padding(.vertical, 12)
            }
        }
        .background(Backdrop()).foregroundStyle(look.palette.foreground)
    }
}

internal struct RulesScreen: View {
    @Bindable internal var store: StoreOf<RulesFeature>
    internal let name: String
    internal let folder: String?
    internal let agent: AgentOption?

    internal var body: some View {
        ZStack(alignment: .bottom) {
            RulesPage(
                name: name, rules: store.shown, agent: agent, back: { store.showing = false },
                add: { store.send(.add(folder: folder)) }, remove: { store.send(.remove($0)) })
            if store.draft != nil || store.removing != nil {
                Dim().onTapGesture { store.send(.cancel) }
            }
            if let draft = Binding($store.draft) {
                AddRuleSheet(
                    draft: draft, folder: folder, error: store.error, busy: store.busy, save: { store.send(.save) },
                    cancel: { store.send(.cancel) })
            } else if let rule = store.removing {
                RemoveSheet(rule: rule, confirm: { store.send(.confirm) }, cancel: { store.send(.cancel) })
            }
        }
        .disabled(store.busy)
    }
}
