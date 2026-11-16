import CooeeCore
import Foundation
import Testing

internal struct TerminalTests {
    private static func update(_ json: String) throws -> TerminalUpdate {
        try JSONDecoder().decode(TerminalUpdate.self, from: Data(json.utf8))
    }

    @Test internal func aFullUpdateDrawsEveryRowAndTheCursor() throws {
        var screen = TerminalScreen()
        screen.apply(
            try Self.update(
                #"{"seq":1,"full":true,"rows":3,"cols":20,"cursor":[1,4],"scrollback":0,"lines":["#
                    + #"{"row":0,"runs":[{"text":"$ ls","fg":2,"bg":null,"bold":true,"italic":false,"#
                    + #""underline":false,"inverse":false}]},{"row":1,"runs":[{"text":"$ "},{"text":"x"}]}]}"#))
        #expect(screen.lines == ["$ ls", "$ x ▍", ""])
    }

    @Test internal func aPartialUpdateChangesOnlyItsRows() throws {
        var screen = TerminalScreen()
        screen.apply(
            try Self.update(#"{"full":true,"rows":2,"cursor":[0,0],"lines":[{"row":1,"runs":[{"text":"b"}]}]}"#))
        screen.apply(
            try Self.update(#"{"full":false,"rows":2,"cursor":[1,0],"lines":[{"row":0,"runs":[{"text":"a"}]}]}"#))
        #expect(screen.lines == ["a", "▍"])
    }

    @Test internal func keystrokesNeverShowInADescriptionOrADump() {
        let typed = Keystrokes("hunter2")
        var dumped = ""
        dump(typed, to: &dumped)
        for shown in [String(describing: typed), String(reflecting: typed), dumped, "\(typed)"] {
            #expect(!shown.contains("hunter2"))
        }
        #expect(typed.text == "hunter2")
    }
}
