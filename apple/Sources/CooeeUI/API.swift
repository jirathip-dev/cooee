import Foundation

// #143: the client for docs/api.md: pairing, threads, the SSE stream with catch-up, and sending. URLSession only.

/// A paired server: its address and the device's `cooee` cookie (kept in the Keychain).
public struct Server: Codable, Sendable, Equatable {
    public var url: URL
    public var token: String
    public init(url: URL, token: String) { self.url = url; self.token = token }
}

/// A failure: the status, and cooee's own sentence when the answer carries `Cooee-Error: 1`.
public struct APIError: Error, LocalizedError, Equatable {
    public let status: Int
    public let text: String
    public var errorDescription: String? { text.isEmpty ? "The server answered \(status)." : text }
}

/// `GET /threads`' row.
public struct ThreadRow: Codable, Sendable, Equatable, Identifiable {
    public let id: String
    public let title: String
    public let agent: String
    public let persona: String
    public let state: String
    public let last: String
    public let at: Int64
    public let unseen: Int
    public let archived: Bool
}

/// A persona (#126).
public struct APIPersona: Codable, Sendable, Equatable, Identifiable {
    public let id: String
    public let name: String
    public let agent: String
    public let model: String
    public let effort: String
    public let folder: String
    public let character: String
    public let instructions: String
    public let plugins: [String]
}

/// One row of a thread (docs/api.md `Item`); kinds this build doesn't draw keep their `kind` only.
public struct APIItem: Codable, Sendable, Equatable, Identifiable {
    public let id: Int
    public let at: Int64
    public let kind: String
    public var text: String?
    public var files: [String]?
    public var title: String?
    public var status: String?
    public var tool: String?
}

public struct Turn: Codable, Sendable, Equatable {
    public var text: String?
    public var state: String
    public var note: String?
    public init(text: String? = nil, state: String = "", note: String? = nil) {
        self.text = text; self.state = state; self.note = note
    }
}

public struct Queued: Codable, Sendable, Equatable {
    public let text: String
    public let random_id: String
}

/// `GET /threads/{id}/messages`.
public struct Slice: Codable, Sendable, Equatable {
    public let items: [APIItem]
    public let turn: Turn
    public let cursor: String
    public let queue: [Queued]
}

/// A file sent with a message: its bytes go as base64.
public struct Attachment: Codable, Sendable, Equatable {
    public let name: String
    public let data: String
    public init(name: String, bytes: Data) { self.name = name; data = bytes.base64EncodedString() }
}

/// One event of a thread's stream.
public enum StreamEvent: Sendable, Equatable {
    case item(APIItem)
    case text(String)
    case state(Turn)
    case queue([Queued])
    /// Too far behind, or another run: load the newest slice again.
    case reset
}

/// SSE framing (`id:`, `event:`, `data:`, a blank line ends one), fed line by line.
struct SSEParser {
    var id = "", event = "", data: [String] = []
    /// The event the line completes, with its id.
    mutating func feed(_ line: Substring) -> (id: String, event: String, data: String)? {
        if line.isEmpty {
            defer { event = ""; data = [] }
            return data.isEmpty && event.isEmpty ? nil : (id, event.isEmpty ? "message" : event, data.joined(separator: "\n"))
        }
        if line.hasPrefix(":") { return nil }
        let (k, v) = line.firstIndex(of: ":").map { (line[..<$0], line[line.index(after: $0)...]) } ?? (line, "")
        let value = String(v.hasPrefix(" ") ? v.dropFirst() : v)
        switch k {
        case "id": id = value
        case "event": event = value
        case "data": data.append(value)
        default: break
        }
        return nil
    }
}

/// Stops a pairing answer's 303 from being followed, so its `Set-Cookie` can be read.
private final class NoRedirect: NSObject, URLSessionTaskDelegate {
    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse,
                    newRequest request: URLRequest) async -> URLRequest? { nil }
}

public final class Client: Sendable {
    public let base: URL
    let token: String?
    let session: URLSession
    /// How long to wait before the stream reconnects.
    let retry: Duration

    public init(base: URL, token: String? = nil, configuration: URLSessionConfiguration = .ephemeral,
                retry: Duration = .seconds(2)) {
        self.base = base; self.token = token; self.retry = retry
        configuration.httpShouldSetCookies = false
        session = URLSession(configuration: configuration)
    }
    public convenience init(_ server: Server) { self.init(base: server.url, token: server.token) }

    func request(_ method: String, _ path: String, json: (any Encodable)? = nil) throws -> URLRequest {
        var r = URLRequest(url: URL(string: path, relativeTo: base)!)
        r.httpMethod = method
        if let token { r.setValue("cooee=\(token)", forHTTPHeaderField: "Cookie") }
        if let json {
            r.setValue("application/json", forHTTPHeaderField: "Content-Type")
            r.httpBody = try JSONEncoder().encode(json)
        }
        return r
    }

    func send(_ r: URLRequest, follow: Bool = true) async throws -> (Data, HTTPURLResponse) {
        let (data, response) = try await session.data(for: r, delegate: follow ? nil : NoRedirect())
        let http = response as! HTTPURLResponse
        if http.statusCode >= 400 {
            let own = http.value(forHTTPHeaderField: "Cooee-Error") == "1"
            throw APIError(status: http.statusCode, text: own ? String(decoding: data, as: UTF8.self) : "")
        }
        return (data, http)
    }

    func call<T: Decodable>(_ method: String, _ path: String, _ body: (any Encodable)? = nil) async throws -> T {
        try JSONDecoder().decode(T.self, from: try await send(request(method, path, json: body)).0)
    }

    func post(_ path: String, _ body: some Encodable) async throws {
        _ = try await send(request("POST", path, json: body))
    }

    // MARK: Pairing (no cookie)

    /// The `cooee` cookie's value in a `Set-Cookie`.
    static func token(_ response: HTTPURLResponse) -> String? {
        let c = response.value(forHTTPHeaderField: "Set-Cookie") ?? ""
        guard let r = c.range(of: "cooee=") else { return nil }
        return String(c[r.upperBound...].prefix { $0 != ";" })
    }

    /// `POST /pair` with a code (the QR's link carries it): the cookie, or why not.
    public func pair(code: String) async throws -> String {
        var r = try request("POST", "/pair")
        r.setValue("application/x-www-form-urlencoded", forHTTPHeaderField: "Content-Type")
        r.httpBody = Data("code=\(code.filter(\.isNumber))".utf8)
        let (_, http) = try await send(r, follow: false)
        if let t = Self.token(http) { return t }
        let to = http.value(forHTTPHeaderField: "Location") ?? ""
        throw APIError(status: http.statusCode, text: to.contains("bad=expired") ? "That code ran out: get a new one."
            : to.contains("bad=limit") ? "Too many wrong codes: wait a minute." : "That code is wrong.")
    }

    public struct Ask: Codable, Sendable { public let word: String; public let claim: String }
    /// `POST /pair/request`: the check word to show, and the claim to collect the cookie with.
    public func askToPair() async throws -> Ask { try await call("POST", "/pair/request", [String: String]()) }

    public enum Claim: Sendable, Equatable { case paired(String), waiting, declined(String), gone(String) }
    /// `POST /pair/claim`.
    public func claim(_ claim: String) async throws -> Claim {
        do {
            let (_, http) = try await send(request("POST", "/pair/claim", json: ["claim": claim]))
            if http.statusCode == 202 { return .waiting }
            return Self.token(http).map(Claim.paired) ?? .waiting
        } catch let e as APIError where e.status == 403 || e.status == 410 {
            return e.status == 403 ? .declined(e.text) : .gone(e.text)
        }
    }
    public func cancelPair(_ claim: String) async throws { try await post("/pair/cancel", ["claim": claim]) }

    // MARK: Threads

    public func threads() async throws -> [ThreadRow] { try await call("GET", "/threads") }
    public func personas() async throws -> [APIPersona] { try await call("GET", "/personas") }
    public func messages(_ id: String, before: Int = 50) async throws -> Slice {
        try await call("GET", "/threads/\(id)/messages?before=\(before)")
    }

    struct Message: Encodable { let text: String; let random_id: String; let files: [Attachment]? }
    /// `send` between turns, `queue` while one runs; a retry with the same id is stored once.
    public func send(_ id: String, text: String, files: [Attachment] = [], queue: Bool = false,
                     randomID: String = UUID().uuidString) async throws {
        try await post("/threads/\(id)/\(queue ? "queue" : "send")",
                       Message(text: text, random_id: randomID, files: files.isEmpty ? nil : files))
    }
    public func steer(_ id: String, _ randomID: String) async throws { try await post("/threads/\(id)/steer", ["random_id": randomID]) }
    public func unqueue(_ id: String, _ randomID: String) async throws { try await post("/threads/\(id)/unqueue", ["random_id": randomID]) }
    public func cancel(_ id: String) async throws { try await post("/threads/\(id)/cancel", [String: String]()) }
    public func read(_ id: String, upTo item: Int) async throws { try await post("/threads/\(id)/read", ["item": item]) }

    struct NewThread: Encodable { let agent: String; let profile: String; let text: String; let random_id: String; let background = true }
    struct Created: Decodable { let id: String }
    /// `POST /threads` for a persona, with its first message: the new thread's id at once.
    public func newThread(persona: APIPersona, text: String) async throws -> String {
        let c: Created = try await call("POST", "/threads",
                                        NewThread(agent: persona.agent, profile: persona.id, text: text, random_id: UUID().uuidString))
        return c.id
    }

    /// `GET /threads/{id}/events`: the events after `since`. A dropped stream reconnects after `retry` with
    /// `Last-Event-ID`, and the server replays what was missed; a `reset` ends it (load the newest slice again).
    public func events(_ id: String, since: String) -> AsyncThrowingStream<StreamEvent, Error> {
        AsyncThrowingStream { go in
            let task = Task {
                var last = since, first = true
                while !Task.isCancelled {
                    do {
                        var r = try request("GET", "/threads/\(id)/events?since=\(last)")
                        if !first { r.setValue(last, forHTTPHeaderField: "Last-Event-ID") }
                        first = false
                        let (bytes, response) = try await session.bytes(for: r)
                        let status = (response as! HTTPURLResponse).statusCode
                        if status >= 400 { throw APIError(status: status, text: "") }
                        var parser = SSEParser(id: last), line: [UInt8] = []
                        for try await b in bytes {
                            guard b == UInt8(ascii: "\n") else { line.append(b); continue }
                            let text = String(decoding: line, as: UTF8.self)
                            line = []
                            guard let e = parser.feed(Substring(text.hasSuffix("\r") ? String(text.dropLast()) : text)) else { continue }
                            last = e.id
                            guard let event = try Self.decode(e.event, Data(e.data.utf8)) else { continue }
                            go.yield(event)
                            if event == .reset { go.finish(); return }
                        }
                    } catch let e as APIError where e.status < 500 {
                        go.finish(throwing: e); return
                    } catch {}
                    try? await Task.sleep(for: retry)
                }
                go.finish()
            }
            go.onTermination = { _ in task.cancel() }
        }
    }

    static func decode(_ event: String, _ data: Data) throws -> StreamEvent? {
        let d = JSONDecoder()
        struct Chunk: Decodable { let text: String }
        switch event {
        case "item": return .item(try d.decode(APIItem.self, from: data))
        case "text": return .text(try d.decode(Chunk.self, from: data).text)
        case "state": return .state(try d.decode(Turn.self, from: data))
        case "queue": return .queue(try d.decode([Queued].self, from: data))
        case "reset": return .reset
        default: return nil
        }
    }
}
