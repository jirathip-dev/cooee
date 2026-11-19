/// One report-mode note (#376): the screen, the tapped element and what is wrong, as `POST /report` takes it.
public struct Report: Encodable, Sendable, Equatable {
    private enum CodingKeys: String, CodingKey {
        case note, path
        case element = "target"
    }

    public let note: String
    public let path: String
    public let element: String

    public init(note: String, path: String, element: String) {
        self.note = note
        self.path = path
        self.element = element
    }
}
