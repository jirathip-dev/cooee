import Foundation

public struct MachineHealth: Codable, Sendable, Equatable {
    private enum CodingKeys: String, CodingKey {
        case loadPerCPU = "load_per_cpu"
        case memory = "mem_pressure_pct"
        case diskFree = "disk_free_mb"
    }

    public let loadPerCPU: Double
    public let memory: Int
    public let diskFree: Int

    public init(loadPerCPU: Double, memory: Int, diskFree: Int) {
        self.loadPerCPU = loadPerCPU
        self.memory = memory
        self.diskFree = diskFree
    }

    public var summary: String {
        let disk = String(format: "%.1f GB free", Double(diskFree) / 1024)
        return "load \(String(format: "%.2f", loadPerCPU)) · memory \(memory)% · \(disk)"
    }
}

public struct MachineRow: Codable, Sendable, Equatable, Identifiable {
    public enum Dot: Sendable, Equatable {
        case ready, warning, offline
    }

    public let name: String
    public let host: String
    public let online: Bool
    public let health: MachineHealth?
    public let error: String?
    private let kind: String?
    public let provider: String?

    public var id: String { name }
    public var title: String { name.isEmpty ? host : name }
    public var desk: Bool { kind == "desk" }
    public var dot: Dot { !online ? .offline : error == nil ? .ready : .warning }

    public var problems: [String] {
        (online ? [] : ["shed can't reach \(title)"]) + (error.map { [$0] } ?? [])
    }

    public init(
        name: String,
        host: String,
        online: Bool = true,
        health: MachineHealth? = nil,
        error: String? = nil,
        desk: Bool = false,
        provider: String? = nil
    ) {
        self.name = name
        self.host = host
        self.online = online
        self.health = health
        self.error = error
        kind = desk ? "desk" : nil
        self.provider = provider
    }
}

extension [MachineRow] {
    public var computers: [MachineRow] { filter { !$0.desk } }
    public var desks: [MachineRow] { filter(\.desk) }

    public func title(_ name: String) -> String {
        first { $0.name == name }?.title ?? (name.isEmpty ? "this host" : name)
    }
}
