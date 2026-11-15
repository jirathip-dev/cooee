import CooeeCore
import Foundation
import Testing

internal struct AppIconTests {
    private let catalog = URL(filePath: #filePath).deletingLastPathComponent()
        .appending(path: "../../Cooee/Assets.xcassets").standardized

    @Test internal func aFreshInstallShowsTheLaughingKookaburraThePrimaryIcon() throws {
        let fresh = try JSONDecoder().decode(Settings.self, from: Data(#"{"scheme":"system","style":"modern"}"#.utf8))
        #expect(fresh.icon == nil)
        #expect(fresh.appIcon == .primary)
        #expect(AppIcon.primary.asset == "ModernLaughing")
        #expect(Settings().appIcon.alternateName == nil)
    }

    @Test internal func followingShowsTheFirstIconOfTheStylesSet() {
        let shown = LookStyle.allCases.map { Settings(style: $0).appIcon.asset }
        #expect(shown == ["ModernLaughing", "TerminalPrompt", "FarmBay"])
    }

    @Test internal func aPickedIconSurvivesAStyleChangeUntilFollowingAgain() throws {
        var settings = Settings()
        settings.icon = IconSet.bush.icons[2]
        settings.style = .terminal
        let kept = try JSONDecoder().decode(Settings.self, from: JSONEncoder().encode(settings))
        #expect(kept.appIcon.alternateName == "BushKoala")
        settings.icon = nil
        #expect(settings.appIcon.asset == "TerminalPrompt")
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
