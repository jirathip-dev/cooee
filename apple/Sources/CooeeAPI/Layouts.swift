import CooeeCore
import Foundation

/// What `GET /layout` and `POST /layout/reset` answer: the served layout and its etag.
public struct LayoutAnswer: Codable, Sendable, Equatable {
    public let etag: String
    public let layout: JSONValue

    public init(etag: String, layout: JSONValue) {
        self.etag = etag
        self.layout = layout
    }
}

/// What `GET /layout/events` says: a good write's etag, or why a bad write was refused.
public enum LayoutEvent: Sendable, Equatable {
    case changed(etag: String)
    case broken(line: Int, why: String)
}

extension Client {
    private struct Broken: Decodable {
        fileprivate let line: Int
        fileprivate let why: String
    }

    private struct Changed: Decodable {
        fileprivate let etag: String
    }

    public func layout(etag: String) async throws -> LayoutAnswer? {
        var request = try request("GET", "/layout")
        request.setValue("\"\(etag)\"", forHTTPHeaderField: "If-None-Match")
        let (data, response) = try await send(request)
        guard response.statusCode != 304 else { return nil }
        return try JSONDecoder().decode(LayoutAnswer.self, from: data)
    }

    public func resetLayout() async throws -> LayoutAnswer {
        try JSONDecoder().decode(LayoutAnswer.self, from: try await send(request("POST", "/layout/reset")).0)
    }

    public func layoutEvents() -> AsyncThrowingStream<LayoutEvent, any Error> {
        AsyncThrowingStream { continuation in
            let task = Task {
                do {
                    let (bytes, response) = try await session.bytes(for: request("GET", "/layout/events"))
                    let status = try Self.http(response).statusCode
                    if status >= 400 { throw APIError(status: status, text: "") }
                    var name = ""
                    for try await line in bytes.lines {
                        if line.hasPrefix("event:") { name = line.dropFirst(6).trimmingCharacters(in: .whitespaces) }
                        guard line.hasPrefix("data:") else { continue }
                        if let event = try Self.layoutEvent(name, Data(line.dropFirst(5).utf8)) {
                            continuation.yield(event)
                        }
                    }
                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }

    private static func layoutEvent(_ name: String, _ data: Data) throws -> LayoutEvent? {
        let decoder = JSONDecoder()
        switch name {
        case "changed": return .changed(etag: try decoder.decode(Changed.self, from: data).etag)
        case "broken":
            let broken = try decoder.decode(Broken.self, from: data)
            return .broken(line: broken.line, why: broken.why)
        default: return nil
        }
    }
}

/// The last good layout on this device, kept so a launch draws it before the server answers.
public struct LayoutFile: Sendable {
    private let file: URL

    public init(file: URL = URL.applicationSupportDirectory.appending(path: "layout.json")) {
        self.file = file
    }

    public func load() -> LayoutAnswer? {
        guard let data = try? Data(contentsOf: file) else { return nil }
        return try? JSONDecoder().decode(LayoutAnswer.self, from: data)
    }

    public func save(_ answer: LayoutAnswer?) {
        guard let answer else {
            try? FileManager.default.removeItem(at: file)
            return
        }
        try? FileManager.default.createDirectory(
            at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        try? JSONEncoder().encode(answer).write(to: file, options: .atomic)
    }
}
