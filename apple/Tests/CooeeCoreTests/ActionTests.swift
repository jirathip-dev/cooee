import CooeeCore
import Foundation
import Testing

internal struct ActionTests {
    private static func decode(_ json: String) throws -> JSONValue {
        try JSONDecoder().decode(JSONValue.self, from: Data(json.utf8))
    }

    @Test internal func aTapSendsItsActionWithTheTapsTheSourcesAndTheGivenValues() throws {
        let row = try Self.decode(#"{"tap": {"action": "open-lane", "values": {"n": 7, "x": 1}}, "values": {"x": 2}}"#)
        let action = BlockAction(action: nil, source: row, values: ["choice": .string("y")])
        #expect(
            action
                == BlockAction(
                    action: "open-lane",
                    values: ["n": .number(7), "x": .number(2), "choice": .string("y")]))
        let button = try Self.decode(#"{"action": "wave"}"#)
        #expect(BlockAction(action: nil, source: button, values: [:])?.action == "wave")
        #expect(BlockAction(action: "answer", source: .null, values: [:])?.action == "answer")
        #expect(BlockAction(action: nil, source: .object([:]), values: [:]) == nil)
    }

    @Test internal func aDestructiveTapIsAlwaysConfirmedTitledWithItsLabel() throws {
        let plain = try Self.decode(#"{"label": "Wave", "action": "wave"}"#)
        let stop = try Self.decode(#"{"label": "Stop", "tone": "bad", "action": "stop"}"#)
        let remove = try Self.decode(
            #"{"label": "Remove…", "style": "plain", "confirm": {"title": "Remove it?", "message": "Lanes stay.", "ok": "Remove", "destructive": true}}"#
        )
        #expect(Confirm(for: plain) == nil)
        let stopping = try #require(Confirm(for: stop))
        #expect(
            stopping.title == "Stop" && stopping.accept == "Stop" && stopping.message == nil && stopping.destructive)
        let removing = try #require(Confirm(for: remove))
        #expect(removing.title == "Remove it?" && removing.accept == "Remove" && removing.message == "Lanes stay.")
        let bare = try Self.decode(#"{"confirm": {}}"#)
        #expect(removing.destructive && Confirm(for: bare)?.accept == "OK")
    }

    @Test internal func aSheetSendsEveryFormsFieldsAndAFieldsOwnValueWins() throws {
        let sheet = try Self.decode(
            #"""
            {"type": "sheet", "fields": [{"name": "repo", "value": "a/b"}], "blocks": [
              {"type": "form", "fields": [{"name": "drive", "value": true}, {"name": "repo", "value": "c/d"}, {"name": "KEY"}]},
              {"type": "text", "markdown": "x"}]}
            """#
        )
        #expect(sheet.fieldValues == ["repo": .string("a/b"), "drive": .bool(true)])
        #expect(sheet.string("type") == "sheet" && sheet.list("blocks").count == 2 && sheet.list("none").isEmpty)
    }
}
