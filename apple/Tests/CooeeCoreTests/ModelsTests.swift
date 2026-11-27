@testable import CooeeCore
import Testing

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

    @Test internal func aServerPersonaGetsInitialsATintAndItsIDAsItsSpriteSeed() {
        let persona = Persona(manager, state: .needs)
        #expect(persona.look == PersonaLook(initials: "AM", tint: .cyan))
        #expect(persona.seed == "am" && persona.with(state: .done).seed == "am")
        #expect(persona.id == "am" && persona.name == "Account Manager" && persona.figure == "(!)")
        #expect(persona.state.word == "needs you" && persona.pose == "needs")
        #expect(Persona(coder).look.initials == "CO")
        #expect(persona.with(state: .done).state == .done && persona.with(state: .done).look == persona.look)
    }

    @Test internal func anAgentKeepsItsMascotUntilThePersonaSheetPicksAnother() {
        let hashed = Persona(manager).mascot
        #expect(Persona(manager, seed: "am").mascot == hashed && Persona(manager).with(state: .fail).mascot == hashed)
        let seeds = (0..<64).map { "agent-\($0)" }
        #expect(Set(seeds.map { Mascot(avatar: nil, seed: $0) }) == Set(Mascot.allCases))
        var picked = manager
        for mascot in Mascot.allCases {
            picked.avatar = mascot.avatar
            #expect(Persona(picked).mascot == mascot)
        }
        for stale in ["data:image/png;base64,AA==", "initials-green", "bird-emu"] {
            picked.avatar = stale
            #expect(Persona(picked).mascot == hashed)
        }
        #expect(Mascot.fairyWren.name == "Fairy-wren" && Mascot.fairyWren.avatar == "bird-fairy-wren")
        #expect(Set(seeds.map(Mascot.phase)).count > 32 && seeds.allSatisfy { (0..<1).contains(Mascot.phase($0)) })
    }

    @Test internal func everyTerminalFaceIsItsOwnFiveColumnsAndItsStatesTimeTheirFrames() {
        let faces = Mascot.allCases.map(\.face)
        #expect(Set(faces.map(\.name)).count == 8 && Set(faces.map { $0.text(.idle) }).count == 8)
        let kook = Mascot.kookaburra.face
        #expect(
            kook.text(.idle) == "(°v°)" && kook.text(.idle, at: 2.25) == "(-v-)" && kook.text(.idle, at: 5.5) == "(°v°)"
        )
        #expect(kook.text(.idle, at: 4.8) == "(.v.)" && kook.text(.idle, at: 5.45) == "(°v°)")
        #expect(
            Mascot.fairyWren.face.frames(.working).map(\.text).prefix(5) == [
                "[o-o]", "[o\\o]", "[o|o]", "[o/o]", "[°-°]",
            ])
        #expect(kook.text(.working, at: 0.33) == "(°/°)" && kook.text(.working, at: 0.88) == "(°-°)")
        #expect(kook.text(.done) == "(^v^)" && kook.text(.done, at: 0.6) == "\\^v^/" && kook.text(.merged) == "(^v^)")
        #expect(kook.frames(.needs).map(\.text) == ["!OoO)", "(OoO!", "(OoO)", "!OoO)"])
        #expect(kook.text(.fail) == "(xvx)" && kook.text(.fail, at: 1.9) == "(x.x)")
        for face in faces {
            for state in [LaneState.idle, .working, .done, .needs, .fail] {
                #expect(face.frames(state).allSatisfy { $0.text.count == 5 && $0.hold > 0 })
            }
        }
    }

    @Test internal func aTurnIsWorkingWhileItRunsOrConnects() {
        #expect(Turn(state: "working").working && Turn(state: "connecting").working)
        #expect(!Turn(state: "done").working && !Turn().working)
    }
}
