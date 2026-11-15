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
        return try pixels(Avatar(persona: Persona(persona), size: 92))
    }

    private static func png(side: CGFloat) throws -> Data {
        let image = try #require(ImageRenderer(content: Color.red.frame(width: side, height: side)).cgImage)
        let out = NSMutableData()
        let target = try #require(CGImageDestinationCreateWithData(out, UTType.png.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(target, image, nil)
        #expect(CGImageDestinationFinalize(target))
        return out as Data
    }

    private static func sheet(_ notes: [Field]) throws -> Pixels {
        let sheet = PersonaSheet(
            draft: .constant(chief), choices: PersonaChoices(), memory: PersonaMemory(notes: notes, count: 5),
            error: nil, actions: PersonaActions()
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

    @Test internal func initialsAndAColourDrawInsteadOfTheSymbol() throws {
        let plain = try Self.avatar(nil)
        let green = try Self.avatar(Tint.green.avatar)
        #expect(green.differing(from: plain, by: 16) > 0.01)
        #expect(try Self.avatar(Tint.amber.avatar).differing(from: green, by: 16) > 0.01)
    }

    @Test internal func memoryDrawsOnlyTheNewestThreeNotes() throws {
        let newest = (1...3).map { Field("Note \($0)", "from you · \($0)d") }
        let five = try Self.sheet(newest + [Field("Old 4", "4d"), Field("Old 5", "5d")])
        #expect(
            try Self.sheet(newest + [Field("Other 4", "6d"), Field("Other 5", "7d")]).differing(from: five, by: 16)
                < 0.0001)
        #expect(
            try Self.sheet([Field("Changed", "1m")] + newest.dropFirst() + newest.prefix(2)).differing(
                from: five, by: 16) > 0.001)
    }
}
