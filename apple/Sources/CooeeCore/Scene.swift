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
        PersonaLook.coat(String(title.split(separator: " ").first ?? ""))
    }

    public static func placed(_ lanes: [JSONValue]) -> [Self] {
        let all = lanes.map(Self.init)
        return [LaneState.needs, .fail, .working, .done, .idle].flatMap { state in all.filter { $0.state == state } }
    }
}
