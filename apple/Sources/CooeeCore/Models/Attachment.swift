import Foundation

public struct Attachment: Codable, Sendable, Equatable {
    public let name: String
    public let data: String

    public init(name: String, bytes: Data) {
        self.name = name
        data = bytes.base64EncodedString()
    }
}
