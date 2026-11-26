import Foundation

extension Client {
    private struct NewRoom: Encodable {
        private enum CodingKeys: String, CodingKey {
            case title, members
            case randomID = "random_id"
        }

        fileprivate let title: String
        fileprivate let members: [String]
        fileprivate let randomID: String
    }

    private struct Seat: Encodable {
        private enum CodingKeys: String, CodingKey {
            case persona, remove
            case place = "at"
        }

        fileprivate let persona: String
        fileprivate let place: Int?
        fileprivate let remove: Bool
    }

    public func newRoom(title: String, members: [String], randomID: String = UUID().uuidString) async throws
        -> String
    {
        let created: Created = try await call(
            "POST", "/rooms", NewRoom(title: title, members: members, randomID: randomID))
        return created.id
    }

    public func seat(_ room: String, persona: String, place: Int? = nil, remove: Bool = false) async throws
        -> [String]
    {
        try await call("POST", "/threads/\(room)/members", Seat(persona: persona, place: place, remove: remove))
    }
}
