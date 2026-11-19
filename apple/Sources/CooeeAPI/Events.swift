import CooeeCore
import Foundation

internal struct SSEFrame: Equatable {
    internal let id: String
    internal let event: String
    internal let data: String
}

internal struct SSEParser {
    internal var id = ""
    private var event = ""
    private var data: [String] = []

    internal init(id: String) {
        self.id = id
    }

    internal mutating func feed(_ line: Substring) -> SSEFrame? {
        if line.isEmpty {
            defer {
                event = ""
                data = []
            }
            guard !data.isEmpty || !event.isEmpty else { return nil }
            return SSEFrame(id: id, event: event.isEmpty ? "message" : event, data: data.joined(separator: "\n"))
        }
        guard !line.hasPrefix(":") else { return nil }
        let colon = line.firstIndex(of: ":")
        let key = colon.map { line[..<$0] } ?? line
        let raw = colon.map { line[line.index(after: $0)...] } ?? ""
        let value = String(raw.hasPrefix(" ") ? raw.dropFirst() : raw)
        switch key {
        case "id": id = value
        case "event": event = value
        case "data": data.append(value)
        default: break
        }
        return nil
    }
}

extension Client {
    private struct Chunk: Decodable {
        fileprivate let text: String
    }

    private struct Seat: Decodable {
        fileprivate let member: String
    }

    private struct Stop: Decodable {
        fileprivate let reason: String
    }

    public func events(_ id: String, since: String) -> AsyncThrowingStream<StreamEvent, any Error> {
        AsyncThrowingStream { continuation in
            let task = Task {
                var last = since
                var attempt = 0
                var down = false
                while !Task.isCancelled {
                    attempt += 1
                    do {
                        if try await stream(id, last: &last, resume: attempt > 1, down: &down, into: continuation) {
                            return
                        }
                    } catch let error as APIError where error.status < 500 {
                        continuation.finish(throwing: error)
                        return
                    } catch {
                        down = true
                        continuation.yield(.sync(Self.down(error)))
                    }
                    try? await clock.sleep(for: retry)
                }
                continuation.finish()
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }

    private func stream(
        _ id: String,
        last: inout String,
        resume: Bool,
        down: inout Bool,
        into continuation: AsyncThrowingStream<StreamEvent, any Error>.Continuation
    ) async throws -> Bool {
        var request = try request("GET", "/threads/\(id)/events?since=\(last)")
        if resume { request.setValue(last, forHTTPHeaderField: "Last-Event-ID") }
        let (bytes, response) = try await session.bytes(for: request)
        try Self.open(response)
        if down { continuation.yield(.sync(.live)) }
        down = false
        return try await frames(bytes, id: last) { frame in
            last = frame.id
            guard let event = try Self.decode(frame.event, Data(frame.data.utf8)) else { return false }
            continuation.yield(event)
            guard event == .reset else { return false }
            continuation.finish()
            return true
        }
    }

    private static func down(_ error: any Error) -> Sync {
        (error as? URLError)?.code == .notConnectedToInternet ? .offline : .reconnecting
    }

    private static func open(_ response: URLResponse) throws {
        let status = try http(response).statusCode
        if status >= 400 { throw APIError(status: status, text: "") }
    }

    internal func sse(_ request: URLRequest, id: String, _ each: (SSEFrame) throws -> Bool) async throws -> Bool {
        let (bytes, response) = try await session.bytes(for: request)
        try Self.open(response)
        return try await frames(bytes, id: id, each)
    }

    private func frames(
        _ bytes: URLSession.AsyncBytes, id: String, _ each: (SSEFrame) throws -> Bool
    ) async throws -> Bool {
        var parser = SSEParser(id: id)
        var line: [UInt8] = []
        for try await byte in bytes {
            guard byte == UInt8(ascii: "\n") else {
                line.append(byte)
                continue
            }
            defer { line = [] }
            guard let frame = parser.feed(Self.line(line)) else { continue }
            if try each(frame) { return true }
        }
        return false
    }

    private static func line(_ bytes: [UInt8]) -> Substring {
        let text = String(bytes: bytes, encoding: .utf8) ?? ""
        return Substring(text.hasSuffix("\r") ? String(text.dropLast()) : text)
    }

    private static let decoder = JSONDecoder()

    internal static func decode(_ event: String, _ data: Data) throws -> StreamEvent? {
        switch event {
        case "item": return .item(try decoder.decode(APIItem.self, from: data))
        case "text": return .text(try decoder.decode(Chunk.self, from: data).text)
        case "state": return .state(try decoder.decode(Turn.self, from: data))
        case "queue": return .queue(try decoder.decode([Queued].self, from: data))
        case "reset": return .reset
        default: return try room(event, data)
        }
    }

    private static func room(_ event: String, _ data: Data) throws -> StreamEvent? {
        switch event {
        case "room-turn": return .roomTurn(try decoder.decode(Seat.self, from: data).member)
        case "room-stop": return .roomStop(try decoder.decode(Stop.self, from: data).reason)
        case "room-ask": return .roomAsk(try decoder.decode(RoomAsk.self, from: data))
        default: return nil
        }
    }
}
