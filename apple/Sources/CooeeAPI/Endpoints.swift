import CooeeCore
import Foundation

extension Client {
    private struct Message: Encodable {
        private enum CodingKeys: String, CodingKey {
            case text, files
            case randomID = "random_id"
        }

        fileprivate let text: String
        fileprivate let randomID: String
        fileprivate let files: [Attachment]?
    }

    private struct NewThread: Encodable {
        private enum CodingKeys: String, CodingKey {
            case agent, profile, text, background
            case randomID = "random_id"
        }

        fileprivate let agent: String
        fileprivate let profile: String
        fileprivate let text: String
        fileprivate let randomID: String
        fileprivate let background = true
    }

    private struct Answer: Encodable {
        fileprivate let item: Int
        fileprivate let option: String
        fileprivate let value: String?
    }

    private struct Created: Decodable {
        fileprivate let id: String
    }

    internal static func token(_ response: HTTPURLResponse) -> String? {
        let cookie = response.value(forHTTPHeaderField: "Set-Cookie") ?? ""
        guard let range = cookie.range(of: "cooee=") else { return nil }
        return String(cookie[range.upperBound...].prefix { $0 != ";" })
    }

    public func pair(code: String) async throws -> String {
        var request = try request("POST", "/pair")
        request.setValue("application/x-www-form-urlencoded", forHTTPHeaderField: "Content-Type")
        request.httpBody = Data("code=\(code.filter(\.isNumber))".utf8)
        let (_, http) = try await send(request, follow: false)
        if let token = Self.token(http) { return token }
        let location = http.value(forHTTPHeaderField: "Location") ?? ""
        throw APIError(status: http.statusCode, text: Self.pairRefusal(location))
    }

    internal static func pairRefusal(_ location: String) -> String {
        if location.contains("bad=expired") { return "That code ran out: get a new one." }
        if location.contains("bad=limit") { return "Too many wrong codes: wait a minute." }
        return "That code is wrong."
    }

    public func askToPair() async throws -> PairAsk {
        try await call("POST", "/pair/request", [String: String]())
    }

    public func claim(_ claim: String) async throws -> PairClaim {
        do {
            let (_, http) = try await send(request("POST", "/pair/claim", json: ["claim": claim]))
            if http.statusCode == 202 { return .waiting }
            return Self.token(http).map(PairClaim.paired) ?? .waiting
        } catch let error as APIError where error.status == 403 {
            return .declined(error.text)
        } catch let error as APIError where error.status == 410 {
            return .gone(error.text)
        }
    }

    public func cancelPair(_ claim: String) async throws {
        try await post("/pair/cancel", ["claim": claim])
    }

    public func threads() async throws -> [ThreadRow] { try await call("GET", "/threads") }

    public func personas() async throws -> [APIPersona] { try await call("GET", "/personas") }

    public func messages(_ id: String, before: Int = 50) async throws -> Slice {
        try await call("GET", "/threads/\(id)/messages?before=\(before)")
    }

    public func send(
        _ id: String,
        text: String,
        files: [Attachment] = [],
        queue: Bool = false,
        randomID: String = UUID().uuidString
    ) async throws {
        let message = Message(text: text, randomID: randomID, files: files.isEmpty ? nil : files)
        try await post("/threads/\(id)/\(queue ? "queue" : "send")", message)
    }

    public func steer(_ id: String, _ randomID: String) async throws {
        try await post("/threads/\(id)/steer", ["random_id": randomID])
    }

    public func unqueue(_ id: String, _ randomID: String) async throws {
        try await post("/threads/\(id)/unqueue", ["random_id": randomID])
    }

    public func cancel(_ id: String) async throws {
        try await post("/threads/\(id)/cancel", [String: String]())
    }

    public func answer(_ id: String, item: Int, option: String, value: String?) async throws {
        try await post("/threads/\(id)/approve", Answer(item: item, option: option, value: value))
    }

    public func read(_ id: String, upTo item: Int) async throws {
        try await post("/threads/\(id)/read", ["item": item])
    }

    public func newThread(persona: APIPersona, text: String, randomID: String = UUID().uuidString) async throws
        -> String
    {
        let body = NewThread(agent: persona.agent, profile: persona.id, text: text, randomID: randomID)
        let created: Created = try await call("POST", "/threads", body)
        return created.id
    }
}
