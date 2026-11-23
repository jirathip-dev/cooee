import CooeeCore
import Foundation

extension Client {
    private struct Message: Encodable {
        private enum CodingKeys: String, CodingKey {
            case text, files, via
            case randomID = "random_id"
        }

        fileprivate let text: String
        fileprivate let randomID: String
        fileprivate let files: [Attachment]?
        fileprivate let via: String?
    }

    private struct NewThread: Encodable {
        private enum CodingKeys: String, CodingKey {
            case agent, profile, model, effort, machine, folder, text, background, setup
            case randomID = "random_id"
        }

        fileprivate let agent: String
        fileprivate let profile: String
        fileprivate let model: String?
        fileprivate let effort: String?
        fileprivate let machine: String
        fileprivate let folder: String
        fileprivate let text: String
        fileprivate let randomID: String
        fileprivate let setup: Bool
        fileprivate let background = true
    }

    internal struct Created: Decodable {
        internal let id: String
    }

    public func send(
        _ id: String,
        text: String,
        files: [Attachment] = [],
        queue: Bool = false,
        voice: Bool = false,
        randomID: String = UUID().uuidString
    ) async throws {
        let message = Message(
            text: text, randomID: randomID, files: files.isEmpty ? nil : files, via: voice ? "voice" : nil)
        try await post("/threads/\(id)/\(queue ? "queue" : "send")", message)
    }

    public func retry(_ id: String) async throws {
        try await post("/threads/\(id)/retry", [String: String]())
    }

    public func restart(_ id: String) async throws {
        try await post("/threads/\(id)/restart", [String: String]())
    }

    public func steer(_ id: String, _ randomID: String) async throws {
        try await post("/threads/\(id)/steer", ["random_id": randomID])
    }

    public func unqueue(_ id: String, _ randomID: String) async throws {
        try await post("/threads/\(id)/unqueue", ["random_id": randomID])
    }

    public func move(_ id: String, to machine: String) async throws {
        try await post("/threads/\(id)/move", ["machine": machine])
    }

    public func cancel(_ id: String) async throws {
        try await post("/threads/\(id)/cancel", [String: String]())
    }

    public func newThread(
        persona: APIPersona,
        machine: String,
        text: String,
        setup: Bool = false,
        randomID: String = UUID().uuidString
    ) async throws -> String {
        let body = NewThread(
            agent: persona.agent, profile: persona.id, model: persona.model.isEmpty ? nil : persona.model,
            effort: persona.effort.isEmpty ? nil : persona.effort, machine: machine, folder: persona.folder, text: text,
            randomID: randomID, setup: setup
        )
        let created: Created = try await call("POST", "/threads", body)
        return created.id
    }
}
