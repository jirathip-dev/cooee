import Foundation

public struct SceneLane: Identifiable, Sendable, Equatable {
    public let id: String
    public let title: String
    public let stage: String
    public let state: LaneState
    public let source: JSONValue

    public var name: String { stage.isEmpty ? title : "\(title) \(stage)" }
    public var sprite: String { "horse-\(Self.coat(title))-\(state.pose)" }

    public init(_ lane: JSONValue) {
        source = lane
        id = lane.id ?? ""
        title = lane.string("title") ?? ""
        stage = lane.string("stage") ?? ""
        state = LaneState(rawValue: lane.string("state") ?? "") ?? .idle
    }

    public static func coat(_ title: String) -> String {
        Cast.farm.pick(String(title.split(separator: " ").first ?? ""))
    }

    public static func placed(_ lanes: [JSONValue]) -> [Self] {
        let all = lanes.map(Self.init)
        return [LaneState.needs, .fail, .working, .done, .idle].flatMap { state in all.filter { $0.state == state } }
    }
}

public struct LaneHead: Sendable, Equatable {
    public let thread: String
    public let lane: SceneLane?
    public let round: JSONValue

    public var line: String {
        let lane = round["lane"]
        return ["harness", "model", "host"].compactMap { lane?.string($0) }.joined(separator: " · ")
    }

    public var pull: JSONValue? { round.list("links").first { $0.string("label")?.hasPrefix("PR") == true } }

    public init(thread: String, screen: JSONValue) {
        let all = screen.tree
        self.thread = thread
        lane = all.filter { $0.type == "scene" }.flatMap { $0.list("lanes") }
            .first { $0["tap"]?.string("thread") == thread }.map(SceneLane.init)
        round = all.first { $0["lane"]?.string("name") == thread } ?? .null
    }
}

extension JSONValue {
    fileprivate var tree: [Self] { [self] + (items + fields.values).flatMap(\.tree) }
}

/// A scene's weather from its host's load over its limit: clear under half, cloudy from there, a storm at the limit.
public enum Weather: String, Sendable, Equatable {
    case clear, cloudy, storm

    public init(load: Double?) {
        switch load ?? 0 {
        case ..<0.5: self = .clear
        case ..<1: self = .cloudy
        default: self = .storm
        }
    }

    /// Night from 19:00 to 07:00 on the local clock.
    public static func night(at date: Date, in calendar: Calendar = .current) -> Bool {
        let hour = calendar.component(.hour, from: date)
        return hour >= 19 || hour < 7
    }
}
