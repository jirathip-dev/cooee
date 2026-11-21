import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct RulesFeatureTests {
    private let calls = Calls()
    private let starter = Rule(
        id: "01", effect: .allow, kind: .read, pattern: "**", folder: "/w", source: "starter",
        words: "Allow reading any file in /w")
    private let deny = Rule(
        id: "a1", effect: .deny, kind: .tool, pattern: "github/delete_repo", source: "settings",
        words: "Deny using github/delete_repo")

    private func store() -> TestStoreOf<RulesFeature> {
        let (calls, starter, deny) = (calls, starter, deny)
        return TestStore(initialState: RulesFeature.State(server: mini, persona: "coder")) {
            RulesFeature()
        } withDependencies: {
            $0.personas.rules = { _, persona in
                calls.record("list \(persona)")
                return [starter]
            }
            $0.personas.addRule = { _, persona, rule in
                calls.record("add \(persona) \(rule.effect) \(rule.kind) \(rule.pattern) \(rule.folder ?? "any")")
                let refused = APIError(status: 400, text: "a rule needs a one-line pattern")
                guard !rule.pattern.isEmpty else { throw refused }
                return deny
            }
            $0.personas.removeRule = { _, persona, id in calls.record("remove \(persona) \(id)") }
        }
    }

    @Test internal func aRuleAddedInSettingsIsPostedAndListedStrictestFirst() async {
        let store = store()
        await store.send(.open) { $0.showing = true }
        await store.receive(\.listed) { $0.rules = [self.starter] }
        await store.send(.add(folder: "/w")) { $0.draft = NewRule(folder: "/w") }
        await store.send(.binding(.set(\.draft, NewRule(effect: .deny, kind: .tool, pattern: "github/delete_repo")))) {
            $0.draft = NewRule(effect: .deny, kind: .tool, pattern: "github/delete_repo")
        }
        await store.send(.save) { $0.busy = true }
        await store.receive(\.saved) {
            ($0.busy, $0.draft, $0.rules) = (false, nil, [self.starter, self.deny])
        }
        #expect(store.state.shown == [deny, starter])
        #expect(calls.all == ["list coder", "add coder deny tool github/delete_repo any"])
    }

    @Test internal func aRefusedRuleKeepsTheSheetOpenWithTheServersWords() async {
        let store = store()
        await store.send(.add(folder: nil)) { $0.draft = NewRule() }
        await store.send(.save) { $0.busy = true }
        await store.receive(\.failed) { ($0.busy, $0.error) = (false, "a rule needs a one-line pattern") }
        #expect(store.state.draft == NewRule())
        await store.send(.cancel) { ($0.draft, $0.error) = (nil, nil) }
    }

    @Test internal func removingARuleAsksFirstThenDeletesItOnTheServer() async {
        var state = RulesFeature.State(server: mini, persona: "coder")
        state.rules = [starter, deny]
        let store = TestStore(initialState: state) {
            RulesFeature()
        } withDependencies: { [calls] in
            $0.personas.removeRule = { _, persona, id in calls.record("remove \(persona) \(id)") }
        }
        await store.send(.remove(deny)) { $0.removing = self.deny }
        #expect(calls.all.isEmpty)
        await store.send(.confirm) { $0.busy = true }
        await store.receive(\.removed) { ($0.busy, $0.removing, $0.rules) = (false, nil, [self.starter]) }
        #expect(calls.all == ["remove coder a1"])
    }
}
