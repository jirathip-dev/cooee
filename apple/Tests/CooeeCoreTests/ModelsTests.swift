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
        #expect(persona.look == PersonaLook(initials: "AM", symbol: "person.fill", tint: .cyan))
        #expect(persona.seed == "am" && persona.with(state: .done).seed == "am")
        #expect(persona.id == "am" && persona.name == "Account Manager" && persona.figure == "(!)")
        #expect(persona.state.word == "at the gate" && persona.pose == "needs")
        #expect(Persona(coder).look.initials == "CO")
        #expect(persona.with(state: .done).state == .done && persona.with(state: .done).look == persona.look)
    }

    @Test internal func anInitialsAvatarDrawsItsInitialsInThePickedTint() {
        var picked = manager
        picked.avatar = Tint.green.avatar
        #expect(Persona(picked).look == PersonaLook(initials: "AM", symbol: "", tint: .green))
        picked.avatar = "data:image/png;base64,AA=="
        #expect(Persona(picked).look == Persona(manager).look)
    }

    @Test internal func aTurnIsWorkingWhileItRunsOrConnects() {
        #expect(Turn(state: "working").working && Turn(state: "connecting").working)
        #expect(!Turn(state: "done").working && !Turn().working)
    }
}
