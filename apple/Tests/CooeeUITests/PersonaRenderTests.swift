import CooeeCore
@testable import CooeeUI
import ImageIO
import SwiftUI
import Testing
import UniformTypeIdentifiers

@MainActor
internal struct PersonaRenderTests {
    private static let chief = APIPersona(id: "chief", name: "Chief", agent: "codex")

    private static func pixels(_ view: some View, height: CGFloat = 120) throws -> Pixels {
        let framed = view.frame(width: 402, height: height).environment(\.look, Look(palette: .farmLight, still: true))
        let image = try #require(ImageRenderer(content: framed).cgImage)
        return try #require(Pixels(image))
    }

    private static func avatar(_ picture: String?) throws -> Pixels {
        var persona = chief
        persona.avatar = picture
        return try pixels(FigureView(persona: Persona(persona), size: 92))
    }

    private static func png(side: CGFloat) throws -> Data {
        let image = try #require(ImageRenderer(content: Color.red.frame(width: side, height: side)).cgImage)
        let out = NSMutableData()
        let target = try #require(CGImageDestinationCreateWithData(out, UTType.png.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(target, image, nil)
        #expect(CGImageDestinationFinalize(target))
        return out as Data
    }

    private static func sheet(_ texts: [String]) throws -> Pixels {
        let notes = texts.map { #"{"text": "\#($0)", "by": "owner", "at": 1000}"# }.joined(separator: ",")
        let screen = JSONValue(
            json: #"{"type": "screen", "blocks": [{"type": "memory", "count": 5, "notes": [\#(notes)]}]}"#)
        let sheet = PersonaEditBlocks(
            draft: .constant(chief), choices: PersonaChoices(), screen: screen, error: nil, actions: PersonaActions()
        )
        return try pixels(sheet, height: 1400)
    }

    @Test internal func aPickedPhotoIsShrunkToAJpegAndDrawnAsTheAvatar() throws {
        let picture = try #require(AvatarPicture.dataURL(try Self.png(side: 900)))
        #expect(picture.hasPrefix("data:image/jpeg;base64,"))
        let image = try #require(AvatarPicture.image(picture))
        #expect(max(image.width, image.height) <= 256)
        #expect(AvatarPicture.dataURL(Data("not an image".utf8)) == nil)
        #expect(AvatarPicture.image("https://x/a.png") == nil)
        #expect(try Self.avatar(picture).differing(from: try Self.avatar(nil), by: 16) > 0.01)
    }

    @Test internal func aPickedCharacterDrawsItsBirdInsteadOfTheHashedOne() throws {
        let hashed = Persona(Self.chief).mascot
        let others = Mascot.allCases.filter { $0 != hashed }
        let plain = try Self.avatar(nil)
        #expect(try Self.avatar(hashed.avatar).differing(from: plain, by: 16) == 0)
        let first = try Self.avatar(others[0].avatar)
        #expect(first.differing(from: plain, by: 16) > 0.01)
        #expect(try Self.avatar(others[1].avatar).differing(from: first, by: 16) > 0.01)
    }

    @Test internal func memoryDrawsOnlyTheNewestThreeNotes() throws {
        let newest = ["Note 1", "Note 2", "Note 3"]
        let five = try Self.sheet(["Old 4", "Old 5"] + newest)
        #expect(try Self.sheet(["Other 4", "Other 5"] + newest).differing(from: five, by: 16) < 0.0001)
        let changed = try Self.sheet(["Old 4", "Old 5", "Note 1", "Note 2", "Changed"]).differing(from: five, by: 16)
        let none = try Self.sheet([]).differing(from: five, by: 16)
        #expect(changed > 0.0003 && none > 0.001)
        let all = PersonaEditBlocks.notes(
            JSONValue(
                json:
                    #"{"blocks": [{"type": "memory", "notes": [{"text": "a", "by": "owner", "at": 1}, {"text": "b", "by": "persona", "at": 2}]}]}"#
            ), persona: "Chief", now: Date(timeIntervalSince1970: 10))
        #expect(all.map(\.key) == ["b", "a"])
    }

    private static func others(_ chat: ListedChat, _ theme: String, _ style: Style = .bundled) throws -> Pixels {
        let words = try #require(OthersFrames.themes[theme])
        return try #require(OthersFrames.render([chat], words: words, style: style).flatMap(Pixels.init))
    }

    @Test(arguments: [("farm", "grazing"), ("kitchen", "standing by"), ("garden", "resting"), ("plain", "idle")])
    internal func aWorkflowRowInOthersSaysItsThemesWordForItsState(_ theme: String, _ word: String) throws {
        let said = try Self.others(OthersFrames.lane("#1", .idle), theme)
        #expect(try Self.others(OthersFrames.lane("#1", .idle, status: word), theme).differing(from: said, by: 16) == 0)
        let farm = try Self.others(OthersFrames.lane("#1", .idle, status: "grazing"), theme)
        #expect(theme == "farm" || farm.differing(from: said, by: 16) > 0)
    }

    @Test internal func aWorkflowRowDrawsAFigureOnlyWhereTheLookDrawsSprites() throws {
        let lane = OthersFrames.lane("#1", .idle)
        let initials = PersonaLook(initials: "C", tint: .lead, coat: "")
        let bare = ListedChat(
            row: lane.row,
            persona: Persona(api: lane.persona.api, look: initials, state: .idle, seed: lane.persona.seed),
            workflow: lane.workflow)
        #expect(try Self.others(lane, "plain").differing(from: try Self.others(bare, "plain"), by: 16) == 0)
        #expect(
            try Self.others(lane, "farm", .scene).differing(from: try Self.others(bare, "farm", .scene), by: 16) > 0)
    }
}
