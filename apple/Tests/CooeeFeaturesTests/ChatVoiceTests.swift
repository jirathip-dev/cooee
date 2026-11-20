import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct ChatVoiceTests {
    private let calls = Calls()

    private func store(_ state: ChatFeature.State) -> TestStoreOf<ChatFeature> {
        let calls = calls
        return TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.date.now = epoch
            $0.continuousClock = ImmediateClock()
            $0.threads.send = { _, thread, text, files, _ in calls.record("send \(thread) \(text) \(files.count)") }
            $0.threads.say = { _, thread, text, queue in calls.record("say \(thread) \(text) \(queue)") }
            $0.voice.speak = { calls.record("speak \($0)") }
            $0.voice.hush = { calls.record("hush") }
        }
    }

    private func chat() -> ChatFeature.State {
        ChatFeature.State(server: mini, target: .thread("a1"), persona: Persona(coder))
    }

    @Test internal func aTapDictatesIntoTheDraftAndNeverSendsIt() async {
        let (heard, ear) = AsyncThrowingStream.makeStream(of: Heard.self)
        var state = chat()
        state.draft = "Rerun"
        let store = store(state)
        store.dependencies.voice.dictate = { heard }
        await store.send(.voice(.mic)) { $0.dictation = Dictation(before: "Rerun") }
        let forming = Heard(settled: "the export with the Q3", pending: "ten Q")
        ear.yield(forming)
        await store.receive(\.voice.heard) { $0.dictation?.heard = forming }
        await store.send(.compose(.send))
        let unsure = Unsure(word: "ten Q", alternatives: ["10-Q", "10Q"])
        let settled = Heard(settled: "the export with the Q3 ten Q filing", unsure: [unsure])
        ear.yield(settled)
        await store.receive(\.voice.heard) { $0.dictation?.heard = settled }
        await store.send(.voice(.mic)) {
            $0.draft = "Rerun the export with the Q3 ten Q filing"
            $0.dictation = Dictation(before: "Rerun", heard: settled, listening: false)
        }
        #expect(calls.all.isEmpty)
    }

    @Test internal func aPickedAlternativeTakesTheUnsureWordsPlaceAndSendSendsIt() async {
        var state = chat()
        state.draft = "Rerun the export with the Q3 ten Q filing"
        let heard = Heard(settled: state.draft, unsure: [Unsure(word: "ten Q", alternatives: ["10-Q", "10Q"])])
        state.dictation = Dictation(before: "", heard: heard, listening: false)
        let store = store(state)
        await store.send(.voice(.pick("10-Q"))) {
            $0.draft = "Rerun the export with the Q3 10-Q filing"
            $0.dictation = nil
        }
        await store.send(.compose(.send)) {
            $0.draft = ""
            $0.outbox = [Outgoing(id: -1, text: "Rerun the export with the Q3 10-Q filing")]
        }
        await store.receive(\.outbox.delivered) { $0.outbox = [] }
        #expect(calls.all == ["send a1 Rerun the export with the Q3 10-Q filing 0"])
    }

    @Test internal func editingAwayAnUnsureWordDropsItsAlternatives() async {
        var state = chat()
        state.draft = "the ten Q filing"
        let heard = Heard(settled: "the ten Q filing", unsure: [Unsure(word: "ten Q", alternatives: ["10-Q"])])
        state.dictation = Dictation(before: "", heard: heard, listening: false)
        let store = store(state)
        await store.send(.compose(.draftChanged("the 10-K filing"))) {
            $0.draft = "the 10-K filing"
            $0.dictation = nil
        }
    }

    @Test internal func aHoldRecordsAVoiceNoteIntoTheDraftUnsent() async {
        let note = Attachment(name: "voice-note.m4a", bytes: Data("aac".utf8))
        let store = store(chat())
        let calls = calls
        store.dependencies.voice.record = { calls.record("record") }
        store.dependencies.voice.recorded = { note }
        await store.send(.voice(.hold(true)))
        await store.send(.voice(.hold(false)))
        await store.receive(\.voice.noted) { $0.files = [note] }
        #expect(calls.all == ["record"])
    }

    @Test internal func aCallSendsEachUtteranceAsAVoiceTurnAndSpeaksTheReply() async {
        let (lines, ear) = AsyncThrowingStream.makeStream(of: String.self)
        var state = chat()
        state.items = [APIItem(id: 4, time: 1, kind: "agent", text: "Earlier")]
        let store = store(state)
        store.dependencies.voice.hear = { lines }
        await store.send(.call(.start)) { $0.call = Calling(started: epoch, from: 4, spoken: 4) }
        ear.yield("Why the drop?")
        await store.receive(\.call.said)
        let reply = APIItem(id: 6, time: 9, kind: "agent", text: "It's **deferred**.")
        await store.send(.event(.item(reply))) {
            $0.items.append(reply)
            $0.call?.spoken = 6
            $0.call?.speaking = "It's deferred."
        }
        await store.receive(\.call.spoken) { $0.call?.speaking = nil }
        await store.send(.call(.shown(false))) { $0.call?.shown = false }
        await store.send(.call(.start)) { $0.call?.shown = true }
        await store.send(.call(.end)) { $0.call = nil }
        #expect(calls.all == ["say a1 Why the drop? false", "speak It's deferred.", "hush"])
    }

    @Test internal func unsureWordsAreWhereTheOtherHearingsDiffer() {
        let unsure = Unsure.between(
            "the Q3 ten Q filing", ["the Q3 10-Q filing", "the Q3 10Q filing", "the Q3 ten Q filing"])
        #expect(unsure == [Unsure(word: "ten Q", alternatives: ["10-Q", "10Q"])])
    }
}
