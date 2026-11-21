import ComposableArchitecture
import CooeeAPI
import CooeeCore
@testable import CooeeFeatures
import Foundation
import Testing

@MainActor
internal struct ChatFramesTests {
    private let calls = Calls()

    @Test internal func aFrameLoadsOnceAndOneGoneAfterItsDaysIsNotAskedForAgain() async {
        let calls = calls
        let state = ChatFeature.State(server: mini, target: .thread("a1"), persona: Persona(coder))
        let store = TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.threads.frame = { _, thread, name in
                calls.record("frame \(thread) \(name)")
                guard name == "f1" else { throw URLError(.fileDoesNotExist) }
                return Data("pixels".utf8)
            }
        }
        await store.send(.frame("f1"))
        await store.receive(\.framed) { $0.frames["f1"] = Data("pixels".utf8) }
        await store.send(.frame("f1"))
        await store.send(.frame("gone"))
        await store.receive(\.framed) { $0.frames["gone"] = Data() }
        await store.send(.frame("gone"))
        #expect(calls.all == ["frame a1 f1", "frame a1 gone"])
    }

    @Test internal func aSentPhotoLoadsOnceFromTheThreadsFiles() async {
        let calls = calls
        let state = ChatFeature.State(server: mini, target: .thread("a1"), persona: Persona(coder))
        let store = TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.threads.file = { _, thread, name in
                calls.record("file \(thread) \(name)")
                guard name == "a.png" else { throw URLError(.fileDoesNotExist) }
                return Data("pixels".utf8)
            }
        }
        await store.send(.photo("a.png"))
        await store.receive(\.photoed) { $0.photos["a.png"] = Data("pixels".utf8) }
        await store.send(.photo("a.png"))
        await store.send(.photo("b.png"))
        await store.receive(\.photoed) { $0.photos["b.png"] = Data() }
        #expect(calls.all == ["file a1 a.png", "file a1 b.png"])
    }

    @Test internal func aNewChatHasNoFramesToLoad() async {
        let store = TestStore(
            initialState: ChatFeature.State(server: mini, target: .new(coder), persona: Persona(coder))
        ) {
            ChatFeature()
        }
        await store.send(.frame("f1"))
    }
}

@MainActor
internal struct ChatBrowserTests {
    private let calls = Calls()
    private let start = Date(timeIntervalSince1970: 1_700_000_000)

    private func store(
        _ clock: TestClock<Duration>, _ time: LockIsolated<Date>,
        _ live: AsyncThrowingStream<LiveEvent, any Error> = .finished(), items: [APIItem] = []
    ) -> TestStoreOf<ChatFeature> {
        var state = ChatFeature.State(server: mini, target: .thread("a1"), persona: Persona(coder))
        state.items = items
        return TestStore(initialState: state) {
            ChatFeature()
        } withDependencies: {
            $0.continuousClock = clock
            $0.date = DateGenerator { time.value }
            $0.threads = threads(live)
        }
    }

    private func threads(_ live: AsyncThrowingStream<LiveEvent, any Error>) -> ThreadsClient {
        let (calls, until) = (calls, start.addingTimeInterval(600))
        var threads = ThreadsClient()
        threads.screen = { _, persona in
            calls.record("screen \(persona)")
            return BrowserScreen()
        }
        threads.live = { _, _ in live }
        threads.takeControl = { _, persona in
            calls.record("take \(persona)")
            return BrowserScreen(until: until, waiting: 1)
        }
        threads.resume = { _, thread, note in calls.record("resume \(thread) \(note)") }
        threads.handBack = { _, persona, note in
            calls.record("back \(persona) \(note)")
            return BrowserScreen()
        }
        threads.input = { _, persona, input in
            calls.record("input \(persona) \(input)")
            if case .text = input { throw APIError(status: 400, text: "take control of coder's browser first") }
        }
        return threads
    }

    @Test internal func theSheetShowsEachLiveFrameAndWhyTheBrowserIsGone() async {
        let (live, feed) = AsyncThrowingStream<LiveEvent, any Error>.makeStream()
        let store = store(TestClock(), LockIsolated(start), live)
        let frame = LiveFrame(url: "https://a.com/", title: "A", jpeg: Data("jpeg".utf8))
        await store.send(.browse(.show(true))) { $0.browser = BrowserSheet(now: self.start) }
        let watching = await store.send(.browse(.watch))
        await store.receive(\.browse.screen)
        feed.yield(.frame(frame))
        await store.receive(\.browse.live) { $0.browser?.live = frame }
        feed.yield(.gone("coder's browser isn't running"))
        await store.receive(\.browse.live) { $0.browser?.gone = "coder's browser isn't running" }
        feed.yield(.frame(frame))
        await store.receive(\.browse.live) { $0.browser?.gone = nil }
        await watching.cancel()
        #expect(calls.all == ["screen coder"])
    }

    @Test internal func theLeaseCountsDownAndReturnsToTheAgentAtZero() async {
        let (clock, time) = (TestClock(), LockIsolated(start))
        let store = store(clock, time)
        await store.send(.browse(.show(true))) { $0.browser = BrowserSheet(now: self.start) }
        let watching = await store.send(.browse(.watch))
        await store.receive(\.browse.screen)
        await store.send(.browse(.take))
        await store.receive(\.browse.screen) {
            $0.browser?.screen = BrowserScreen(until: self.start.addingTimeInterval(600), waiting: 1)
        }
        #expect(store.state.browser?.left == 600)
        time.setValue(start.addingTimeInterval(599.5))
        await clock.advance(by: .seconds(1))
        await store.receive(\.browse.tick) { $0.browser?.now = self.start.addingTimeInterval(599.5) }
        #expect(store.state.browser?.left == 1)
        time.setValue(start.addingTimeInterval(600))
        await clock.advance(by: .seconds(1))
        await store.receive(\.browse.tick) {
            $0.browser?.now = self.start.addingTimeInterval(600)
            $0.browser?.screen.until = nil
        }
        #expect(store.state.browser?.held == false)
        await watching.cancel()
        await store.send(.browse(.show(false))) { $0.browser = nil }
        #expect(calls.all == ["screen coder", "take coder"])
    }

    @Test internal func handBackCarriesTheNoteAndARefusedInputSaysWhy() async {
        let store = store(TestClock(), LockIsolated(start))
        await store.send(.browse(.show(true))) { $0.browser = BrowserSheet(now: self.start) }
        await store.send(.browse(.send(.click(left: 10, top: 20))))
        await store.send(.browse(.typed("hunter2"))) { $0.browser?.typed = "hunter2" }
        await store.send(.browse(.send(.text("hunter2")))) { $0.browser?.typed = "" }
        await store.receive(\.browse.failed) { $0.browser?.error = "take control of coder's browser first" }
        await store.send(.browse(.replay("f1"))) { $0.browser?.replay = "f1" }
        await store.send(.browse(.note("signed in"))) { $0.browser?.note = "signed in" }
        await store.send(.browse(.handBack)) { $0.browser?.note = "" }
        await store.receive(\.browse.screen) { $0.browser?.error = nil }
        #expect(
            calls.all == [
                "input coder click(left: 10.0, top: 20.0)", "input coder text(\"hunter2\")", "back coder signed in",
            ])
    }

    @Test internal func aWaitingHandoffResumesWithTheNoteOnHandBackOrDone() async {
        var handoff = APIItem(id: 4, time: 4000, kind: "handoff", text: "Sign in to a.com")
        handoff.outcome = "waiting"
        let store = store(TestClock(), LockIsolated(start), items: [handoff])
        #expect(store.state.handoff == handoff)
        await store.send(.browse(.takeOver)) { $0.browser = BrowserSheet(now: self.start) }
        await store.receive(\.browse.screen) {
            $0.browser?.screen = BrowserScreen(until: self.start.addingTimeInterval(600), waiting: 1)
        }
        await store.send(.browse(.note("signed in"))) { $0.browser?.note = "signed in" }
        await store.send(.browse(.handBack)) { $0.browser?.note = "" }
        await store.receive(\.browse.screen) { $0.browser?.screen = BrowserScreen() }
        await store.send(.browse(.done))
        await store.send(.browse(.open("f2"))) { $0.browser = BrowserSheet(replay: "f2", now: self.start) }
        #expect(Set(calls.all) == ["take coder", "back coder signed in", "resume a1 signed in", "resume a1 "])
    }

    @Test internal func withNoHandoffWaitingHandBackOnlyReturnsTheBrowser() async {
        var closed = APIItem(id: 4, time: 4000, kind: "handoff")
        closed.outcome = "handed_back"
        let store = store(TestClock(), LockIsolated(start), items: [closed])
        await store.send(.browse(.show(true))) { $0.browser = BrowserSheet(now: self.start) }
        await store.send(.browse(.handBack))
        await store.receive(\.browse.screen)
        await store.send(.browse(.done))
        #expect(calls.all == ["back coder "])
    }
}
