import ComposableArchitecture
import CooeeCore
@testable import CooeeFeatures
import Testing

@MainActor
internal struct ChatTopicsTests {
    private let calls = Calls()
    private let clock = TestClock()

    nonisolated private static func topic(_ id: String, _ age: Int64, archived: Bool = false) -> ThreadRow {
        ThreadRow(
            id: id, title: "T \(id)", agent: "codex", persona: "coder", state: "done", last: "",
            updated: 999_000_000 - age * 60_000, unseen: 0, archived: archived, folder: "/home/user/src/\(id)"
        )
    }

    nonisolated private static let seven = (1...7).map { topic("t\($0)", Int64($0)) }

    private func store(_ target: ChatFeature.Target = .thread("t1")) -> TestStoreOf<ChatFeature> {
        let (calls, clock) = (calls, clock)
        return TestStore(initialState: ChatFeature.State(server: mini, target: target, persona: Persona(coder))) {
            ChatFeature()
        } withDependencies: {
            $0.continuousClock = clock
            $0.threads.topics = { _, persona in
                calls.record("topics \(persona)")
                return Self.seven
            }
            $0.threads.retitle = { _, id, title in calls.record("title \(id) \(title)") }
            $0.threads.archive = { _, id, archived in calls.record("archive \(id) \(archived)") }
            $0.threads.start = { _, persona, _, text, _ in
                calls.record("start \(persona.folder) \(text)")
                return "n1"
            }
        }
    }

    private func loaded(_ target: ChatFeature.Target = .thread("t1")) async -> TestStoreOf<ChatFeature> {
        let store = store(target)
        await store.send(.topics(.load))
        await store.receive(\.topics.loaded) { $0.bar?.topics = Topics(Self.seven) }
        return store
    }

    @Test internal func theSeventhTopicFoldsIntoTheListButton() async {
        let store = await loaded()
        #expect(store.state.bar?.topics.shown.map(\.id) == ["t1", "t2", "t3", "t4", "t5", "t6"])
        #expect(store.state.bar?.topics.folded == 1)
        #expect(calls.all == ["topics coder"])
    }

    @Test internal func theOrderHoldsWhileTheChatIsOnScreen() async {
        let store = await loaded()
        let busier = [Self.topic("t8", 0), Self.topic("t7", 0)] + Self.seven.dropLast()
        await store.send(.topics(.loaded(busier))) { $0.bar?.topics.load(busier) }
        #expect(store.state.bar?.topics.shown.map(\.id) == ["t8", "t1", "t2", "t3", "t4", "t5"])
        #expect(store.state.bar?.topics.folded == 2)
    }

    @Test internal func renameSavesTheTitleAndTheChipShowsIt() async {
        let store = await loaded()
        await store.send(.topics(.press("t2"))) { $0.bar?.pressed = "t2" }
        await store.send(.topics(.rename("t2"))) {
            $0.bar?.pressed = nil
            $0.bar?.renaming = "t2"
            $0.bar?.title = "T t2"
            $0.bar?.listing = true
        }
        await store.send(.topics(.title("Release"))) { $0.bar?.title = "Release" }
        await store.send(.topics(.retitle)) {
            $0.bar?.renaming = nil
            $0.bar?.topics.retitle("t2", "Release")
        }
        #expect(store.state.bar?.topics.shown[1].title == "Release")
        #expect(calls.all == ["topics coder", "title t2 Release"])
    }

    @Test internal func archiveThenUndoWithinFiveSecondsKeepsTheTopic() async {
        let store = await loaded()
        await store.send(.topics(.archive("t2"))) {
            $0.bar?.undo = "t2"
            $0.bar?.topics.archive("t2", true)
        }
        #expect(store.state.bar?.topics.shown.map(\.id).contains("t2") == false)
        await clock.advance(by: .seconds(4))
        await store.send(.topics(.undo)) {
            $0.bar?.undo = nil
            $0.bar?.topics.archive("t2", false)
        }
        await clock.advance(by: .seconds(2))
        #expect(store.state.bar?.topics.shown.map(\.id).contains("t2") == true)
        #expect(calls.all == ["topics coder", "archive t2 true", "archive t2 false"])
    }

    @Test internal func anArchivedTopicIsRestoredAfterTheToastGoes() async {
        let store = await loaded()
        await store.send(.topics(.archive("t3"))) {
            $0.bar?.undo = "t3"
            $0.bar?.topics.archive("t3", true)
        }
        await clock.advance(by: .seconds(5))
        await store.receive(\.topics.expired) { $0.bar?.undo = nil }
        await store.send(.topics(.undo))
        #expect(store.state.bar?.topics.archived.map(\.id) == ["t3"])
        await store.send(.topics(.restore("t3"))) { $0.bar?.topics.archive("t3", false) }
        #expect(store.state.bar?.topics.shown.map(\.id).contains("t3") == true)
        #expect(calls.all == ["topics coder", "archive t3 true", "archive t3 false"])
    }

    @Test internal func newWithAPickedFolderStartsTheThreadInIt() async {
        let store = await loaded(.new(coder))
        store.exhaustivity = .off
        #expect(store.state.bar?.recent.first == "/home/user/src/t1")
        await store.send(.topics(.folder("/home/user/src/t2"))) { $0.bar?.folder = "/home/user/src/t2" }
        await store.send(.compose(.draftChanged("Plan it")))
        await store.send(.compose(.send))
        await store.receive(\.delegate.started)
        await store.send(.topics(.folder(nil)))
        await store.send(.compose(.draftChanged("Scratch it")))
        await store.send(.compose(.send))
        await store.receive(\.delegate.started)
        #expect(calls.all == ["topics coder", "start /home/user/src/t2 Plan it", "start scratch Scratch it"])
    }
}

@MainActor
internal struct TopicSwitchTests {
    private func store() -> TestStoreOf<AppFeature> {
        var state = AppFeature.State(layout: testLayout, server: mini)
        state.home?.rows = [row("t1"), row("t2")]
        state.home?.personas = [coder]
        return TestStore(initialState: state) {
            AppFeature()
        } withDependencies: {
            $0.date.now = epoch
        }
    }

    @Test internal func aChipOpensItsThreadAndKeepsTheStrip() async {
        let store = store()
        let rows = [row("t1"), row("t2")]
        await store.send(.open("t1")) {
            $0.path = ["t1"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("t1"), persona: Persona(coder, state: .done))
        }
        await store.send(.chat(.topics(.loaded(rows)))) { $0.chat?.bar?.topics = Topics(rows) }
        await store.send(.chat(.topics(.open("t1"))))
        await store.send(.chat(.topics(.open("t2"))))
        await store.receive(\.chat.delegate.topic) {
            $0.path = ["t2"]
            $0.chat = ChatFeature.State(server: mini, target: .thread("t2"), persona: Persona(coder, state: .done))
            $0.chat?.bar?.topics = Topics(rows)
        }
        await store.send(.chat(.topics(.open(nil))))
        await store.receive(\.chat.delegate.topic) {
            $0.path = ["new:coder"]
            $0.chat = ChatFeature.State(server: mini, target: .new(coder), persona: Persona(coder))
            $0.chat?.bar?.topics = Topics(rows)
        }
    }
}
