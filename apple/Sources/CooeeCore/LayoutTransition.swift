/// How a node comes on screen and goes; `push` when the file names none.
public enum LayoutTransition: String, Sendable {
    case push, sheet, fade, none

    internal init(_ json: JSONValue?, at path: String) throws(LayoutError) {
        guard let json else {
            self = .push
            return
        }
        guard let transition = json.string.flatMap(Self.init(rawValue:)) else {
            throw .badTransition(path: path, transition: json.string ?? "")
        }
        self = transition
    }
}

/// Why a `layout.json` was refused, with the JSON path of the node at fault.
public enum LayoutError: Error, Equatable {
    case format(Int)
    case json(String)
    case unknownType(path: String, type: String)
    case badScreen(path: String)
    case unknownSlot(path: String, slot: String)
    case unknownConfig(path: String, key: String)
    case badPlace(path: String, place: String)
    case badTransition(path: String, transition: String)
    case duplicateId(String)
    case tooDeep(path: String)
    case tooMany(Int)
    case locked(path: String)
}
