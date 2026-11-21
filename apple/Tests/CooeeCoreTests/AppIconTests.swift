import CooeeCore
import Foundation
import Testing

internal struct AppIconTests {
    private let catalog = URL(filePath: #filePath).deletingLastPathComponent()
        .appending(path: "../../Cooee/Assets.xcassets").standardized

    private static func pack(_ name: String) throws -> Style {
        let file = URL(filePath: #filePath).deletingLastPathComponent().appending(
            path: "../../../packs/\(name)/style.json")
        return try JSONDecoder().decode(Style.self, from: Data(contentsOf: file.standardized))
    }

    @Test internal func followingShowsTheFirstIconOfTheKeptStylesSet() throws {
        let picks: [StylePick] = [
            .modern, .pack(name: "terminal", kept: try Self.pack("terminal")),
            .pack(name: "scene", kept: try Self.pack("scene")), .pack(name: "scene", kept: nil),
        ]
        let shown = picks.map { Settings(style: $0).appIcon.asset }
        #expect(shown == ["ModernLaughing", "TerminalPrompt", "FarmBay", "ModernLaughing"])
    }

    @Test internal func aPickedIconAndTheKeptTokensSurviveARelaunchAndAnOldPickNamesItsPack() throws {
        var settings = Settings()
        settings.icon = IconSet.bush.icons[2]
        settings.style = .pack(name: "terminal", kept: try Self.pack("terminal"))
        let kept = try JSONDecoder().decode(Settings.self, from: JSONEncoder().encode(settings))
        #expect(kept.appIcon.alternateName == "BushKoala" && kept.style == settings.style)
        settings.icon = nil
        #expect(settings.appIcon.asset == "TerminalPrompt")
        let old = try JSONDecoder().decode(Settings.self, from: Data(#"{"scheme": "dark", "style": "terminal"}"#.utf8))
        #expect(old.style == .pack(name: "terminal", kept: nil))
    }

    @Test internal func everyIconIsInTheAssetCatalogAsAnAppIconAndAPreview() throws {
        let icons = IconSet.allCases.flatMap(\.icons)
        #expect(icons.count == 24)
        #expect(Set(icons.map(\.asset)).count == 24)
        for icon in icons {
            for set in ["\(icon.alternateName ?? "AppIcon").appiconset", "\(icon.asset)Preview.imageset"] {
                let folder = catalog.appending(path: set)
                let contents = try JSONDecoder().decode(
                    Contents.self, from: Data(contentsOf: folder.appending(path: "Contents.json")))
                let files = contents.images.compactMap(\.filename)
                #expect(!files.isEmpty, "\(set) has no image")
                for file in files {
                    #expect(
                        FileManager.default.fileExists(atPath: folder.appending(path: file).path()), "\(set)/\(file)")
                }
            }
        }
    }
}

private struct Contents: Decodable {
    fileprivate struct Image: Decodable { fileprivate let filename: String? }
    fileprivate let images: [Image]
}
