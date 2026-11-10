import Foundation
import Testing

internal let json = ["Content-Type": "application/json"]
internal let sse = ["Content-Type": "text/event-stream"]

internal enum Fixture {
    internal static func text(_ name: String) throws -> String {
        let url = try #require(Bundle.module.url(forResource: name, withExtension: nil, subdirectory: "Fixtures"))
        return try String(contentsOf: url, encoding: .utf8)
    }

    internal static func answer(_ name: String) throws -> Answer {
        Answer(headers: json, chunks: [try text(name)])
    }
}
