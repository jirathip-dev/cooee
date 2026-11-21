import CooeeCore
import Foundation
import Testing

@MainActor
internal struct DiagramTests {
    private static let folder = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("docs/design/workflow-diagram/fixtures")

    private static func fixture(_ name: String) throws -> JSONValue {
        try JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: folder.appendingPathComponent(name)))
    }

    private static func flow() throws -> JSONValue {
        try #require(try fixture("map.json").list("blocks").first { $0.type == "diagram" })
    }

    @Test internal func drovesMapIsOneSpineWithItsSkippedStatesOptional() throws {
        let diagram = Diagram(source: try Self.flow().string("source") ?? "")
        #expect(diagram.spine == ["queued", "architect", "design", "impl", "review", "verify", "ready", "merged"])
        #expect(diagram.optional == ["architect", "design", "verify"])
        #expect(diagram.name("impl") == "building" && diagram.name("needs_human") == "needs human")
        #expect(diagram.name("fix") == "fix")
    }

    @Test internal func backEdgesGoRightLeavingEdgesGoLeftAndAnOrphanGoesUnderItsPredecessors() throws {
        let diagram = Diagram(source: try Self.flow().string("source") ?? "")
        #expect(diagram.side["blocked"] == 1 && diagram.row["blocked"] == 0)
        #expect(diagram.side["fix"] == 1 && diagram.row["fix"] == 4)
        #expect(diagram.side["stall"] == -1 && diagram.row["stall"] == 3)
        #expect(diagram.side["rounds"] == -1 && diagram.row["rounds"] == 4)
        #expect(diagram.side["needs_human"] == -1 && diagram.row["needs_human"] == 5)
        #expect(diagram.rows == 8)
    }

    @Test internal func aPairBothWaysIsOneLineAndASpineJumpIsNotDrawn() throws {
        let edges = Diagram(source: try Self.flow().string("source") ?? "").edges
        #expect(edges.contains { $0.from == "queued" && $0.into == "blocked" && $0.both })
        #expect(edges.contains { $0.from == "review" && $0.into == "fix" && $0.both })
        #expect(edges.contains { $0.from == "verify" && $0.into == "fix" && !$0.both })
        #expect(!edges.contains { $0.from == "queued" && $0.into == "impl" })
        #expect(!edges.contains { $0.from == "review" && $0.into == "ready" })
        #expect(!edges.contains { $0.from == "[*]" })
    }

    @Test internal func commentsLabelsAndUnknownLinesAreSkipped() {
        let diagram = Diagram(source: "stateDiagram-v2\n  %% a --> z\n  direction LR\n  a --> b : PASS\n  note x\n")
        #expect(diagram.order == ["a", "b"] && diagram.spine == ["a", "b"] && diagram.edges.count == 1)
        #expect(Diagram(source: "").rows == 0)
    }

    @Test internal func aMergeOpUpdatesTheMapsCountsInPlace() throws {
        var screen = LiveScreen(try Self.fixture("map.json"))
        for event in try Self.fixture("stream.json").items { screen.apply(event: event) }
        let flow = try #require(screen.root.list("blocks").first { $0.type == "diagram" })
        #expect(flow["counts"]?["review"] == .number(1) && flow["counts"]?["fix"] == .number(3))
        #expect(flow["source"] == (try Self.flow())["source"])
    }
}
