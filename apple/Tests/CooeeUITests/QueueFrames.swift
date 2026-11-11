import CooeeCore
import CooeeUI
import Foundation
import SwiftUI

internal enum QueueFrames {
    private static let file = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("docs/design/queue-scene/fixtures/queue.json")
    private static let drawnAt = Date(timeIntervalSince1970: 1_794_217_260)

    private static let screen: JSONValue = {
        let raw = (try? JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: file))) ?? .null
        return moved(raw, to: .now)
    }()

    private static func moved(_ value: JSONValue, to now: Date) -> JSONValue {
        switch value {
        case .object(let fields):
            .object(
                fields.reduce(into: [:]) { out, pair in
                    guard pair.key != "other" else { return }
                    let since = pair.value.date.map {
                        JSONValue.string(Relative.stamp(in: $0.timeIntervalSince(drawnAt), from: now))
                    }
                    out[pair.key] = pair.key == "since" ? since ?? pair.value : moved(pair.value, to: now)
                })
        case .array(let items): .array(items.map { moved($0, to: now) })
        default: value
        }
    }

    private static let figures: JSONValue = {
        let tools = ["", #", "tool": "read""#, #", "tool": "run""#].map { #""state": "working"\#($0)"# }
        let figures = tools + ["needs", "fail", "done", "idle"].map { #""state": "\#($0)""# }
        let rows = figures.map { #"{"type": "row", "figure": {\#($0)}}"# }
        let json = #"{"type": "list", "rows": [\#(rows.joined(separator: ","))]}"#
        return (try? JSONDecoder().decode(JSONValue.self, from: Data(json.utf8))) ?? .null
    }()

    @MainActor private static func sheet() -> some View {
        Color.clear.overlay(alignment: .top) { BlocksView(screen).padding(16) }.clipped()
    }

    @MainActor private static func panel() -> some View {
        Color.clear.overlay(alignment: .top) { BlocksView(screen).padding(24).frame(width: 760).padding(.top, 36) }
            .clipped()
    }

    internal static var all: [Frame] {
        let modes: [(String, Palette)] = [("dark", .farmDark), ("light", .farmLight)]
        let phones = modes.flatMap { mode, palette in
            [LookStyle.scene, .modern, .terminal].map { style in
                Frame(
                    name: "queue-\(style.rawValue)-\(mode)", width: 402, height: 874, palette: palette, style: style,
                    phone: true, view: { AnyView(sheet()) })
            }
        }
        let macs = [LookStyle.scene, .modern, .terminal].map { style in
            Frame(
                name: "queue-mac-\(style.rawValue)-dark", width: 1440, height: 900, palette: .farmDark, style: style,
                phone: false, view: { AnyView(panel()) })
        }
        let lanes = Frame(
            name: "queue-figures-modern-dark", width: 64, height: 360, palette: .farmDark, style: .modern,
            phone: false,
            view: { AnyView(Color.clear.overlay(alignment: .top) { BlocksView(figures).padding(.top, 36) }) })
        return phones + macs + [lanes]
    }
}
