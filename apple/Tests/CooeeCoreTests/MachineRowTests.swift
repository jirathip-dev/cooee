import CooeeCore
import Testing

internal struct MachineRowTests {
    private let machines = [
        MachineRow(name: "", host: "mini", health: MachineHealth(loadPerCPU: 0.4219, memory: 61, diskFree: 49357)),
        MachineRow(name: "lab", host: "lab", online: false, error: "link down"),
        MachineRow(name: "d1", host: "d1", desk: true, provider: "cloud"),
    ]

    @Test internal func thisHostIsNamedByItsHostAndThePickerGroupsComputersAndDesks() {
        #expect(machines.computers.map(\.title) == ["mini", "lab"] && machines.desks.map(\.title) == ["d1"])
        #expect(machines.title("") == "mini" && machines.title("lab") == "lab")
        #expect(([MachineRow]()).title("") == "this host" && ([MachineRow]()).title("gone") == "gone")
    }

    @Test internal func theDotAndProblemsSayWhetherAMachineIsReady() {
        #expect(machines.map(\.dot) == [.ready, .offline, .ready])
        #expect(MachineRow(name: "s", host: "s", error: "x").dot == .warning)
        #expect(machines[1].problems == ["shed can't reach lab", "link down"] && machines[0].problems.isEmpty)
        #expect(machines[0].health?.summary == "load 0.42 · memory 61% · 48.2 GB free")
    }
}
