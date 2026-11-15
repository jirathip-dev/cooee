@testable import CooeeAPI
import CooeeCore
import Testing

internal struct MachinesTests {
    @Test internal func machinesDecodeAndAMovePostsTheMachine() async throws {
        let listed = #"""
            [{"name":"","host":"mini","online":true,"health":{"at":1,"load_per_cpu":0.42,"mem_pressure_pct":61,
            "swap_used_mb":0,"disk_free_mb":49357,"cpus":8,"uptime_s":9,"terminals":{}}},
            {"name":"air","host":"air","online":false,"last_seen":5,"error":"link down"},
            {"name":"d1","host":"d1","online":true,"kind":"desk","provider":"cloud"}]
            """#
        let server = CannedServer([
            "/machines": [Answer(headers: json, chunks: [listed])],
            "/threads/a1b2/move": [Answer(headers: json, chunks: ["{}"])],
        ])
        let machines = try await server.client().machines()
        let mini = MachineRow(
            name: "", host: "mini", health: MachineHealth(loadPerCPU: 0.42, memory: 61, diskFree: 49357))
        #expect(machines.computers == [mini, MachineRow(name: "air", host: "air", online: false, error: "link down")])
        #expect(machines.desks == [MachineRow(name: "d1", host: "d1", desk: true, provider: "cloud")])
        try await server.client().move("a1b2", to: "air")
        #expect(try server.decoded("/threads/a1b2/move", as: [String: String].self) == ["machine": "air"])
    }
}
