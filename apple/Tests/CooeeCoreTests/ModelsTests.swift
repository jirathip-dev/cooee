import Foundation
import Testing

@testable import CooeeCore

internal struct ModelsTests {
    private let coder = APIPersona(id: "coder", name: "Coder", agent: "codex", model: "gpt-5.5")
    private let manager = APIPersona(id: "am", name: "Account Manager", agent: "claude")

    @Test(arguments: [
        ("working", LaneState.working), ("connecting", .working), ("blocked", .needs), ("done", .done),
        ("error", .fail), ("unconnected", .fail), ("", .idle), ("ended", .idle),
    ])
    internal func aThreadsStateIsALanesState(_ api: String, _ state: LaneState) {
        #expect(LaneState(api: api) == state)
    }

    @Test internal func eachStateHasItsFigureWordAndPose() {
        let states: [LaneState] = [.working, .needs, .done, .idle, .fail, .merged]
        #expect(states.map(\.figure) == ["~o=", "(!)", "\\o/", "-.-", "x_x", "\\o/"])
        #expect(
            states.map(\.word) == [
                "on the trail", "at the gate", "in the stable", "grazing", "refused", "over the fence",
            ]
        )
        #expect(states.map(\.pose) == ["edit", "needs", "done", "idle", "fail", "merged"])
    }

    @Test internal func aServerPersonaGetsInitialsATintAndACoatByItsID() {
        let persona = Persona(manager, state: .needs)
        #expect(persona.look == PersonaLook(initials: "AM", symbol: "person.fill", tint: .cyan, coat: "gry"))
        #expect(persona.id == "am" && persona.name == "Account Manager" && persona.figure == "(!)")
        #expect(persona.word == "at the gate" && persona.pose == "needs")
        #expect(Persona(coder).look.initials == "CO")
        #expect(persona.with(state: .done).state == .done && persona.with(state: .done).look == persona.look)
    }

    @Test internal func aTurnIsWorkingWhileItRunsOrConnects() {
        #expect(Turn(state: "working").working && Turn(state: "connecting").working)
        #expect(!Turn(state: "done").working && !Turn().working)
    }

    @Test internal func modelsKeepWhatTheyAreGiven() throws {
        let url = try #require(URL(string: "https://mini.local"))
        #expect(Server(url: url, token: "t").token == "t")
        #expect(Attachment(name: "a.txt", bytes: Data("hi".utf8)).data == "aGk=")
        #expect(PairAsk(word: "maple", claim: "c1").claim == "c1")
        #expect(NearbyServer(name: "mini", address: "http://192.0.2.20:4100").id == "mini")
        #expect(Slice(items: [], turn: Turn(), cursor: "b.1", queue: []).cursor == "b.1")
        #expect(StreamEvent.text("a") != .reset)
        #expect(PairClaim.paired("t") != .waiting)
    }
}
