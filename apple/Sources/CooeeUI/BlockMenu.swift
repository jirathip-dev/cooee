import CooeeCore
import SwiftUI

internal struct MenuBlocks: View {
    @Environment(\.look) private var look
    internal let screen: JSONValue
    internal var name = "You"
    internal let place: String
    internal var values: [String: String] = [:]
    internal var pick: (JSONValue) -> Void = { _ in }

    internal var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            ForEach(Array(screen.list("blocks").enumerated()), id: \.offset) { _, block in
                switch block.type {
                case "account": account
                case "group": group(block)
                case "note":
                    Text((look.textMarks ? block.string("mono") : nil) ?? block.string("text") ?? "")
                        .ui(12, .regular, look).foregroundStyle(look.palette.mute).padding(.leading, 6)
                default: EmptyView()
                }
            }
        }
        .padding(14)
        .surface(look, radius: 30)
        .foregroundStyle(look.palette.foreground)
    }

    private var account: some View {
        HStack(spacing: 12) {
            AccountAvatar(size: 48)
            VStack(alignment: .leading, spacing: 2) {
                Text(name).ui(18, .semibold, look)
                Text(place).ui(13, .regular, look).foregroundStyle(look.palette.mute)
            }
            Spacer()
        }
        .padding(.horizontal, 6)
    }

    private func group(_ block: JSONValue) -> some View {
        let rows = block.list("rows")
        return Group2(header: block.string("title") ?? "") {
            ForEach(Array(rows.enumerated()), id: \.offset) { index, row in
                Button {
                    pick(row)
                } label: {
                    SettingRow(
                        symbol: row.string("symbol") ?? "circle", title: row.string("title") ?? "",
                        tint: row.string("tone") == "purple"
                            ? look.palette.purple : look.palette.tone(row.string("tone")),
                        value: values[row.string("id") ?? ""] ?? row.string("value") ?? "",
                        warn: row["warn"]?.bool == true,
                        divider: index < rows.count - 1
                    )
                    .contentShape(.rect)
                }
                .buttonStyle(.plain)
            }
        }
    }
}

extension JSONValue {
    internal static let bundledMenu: JSONValue = {
        // SAFETY: Default/screens/menu.json is copied into this module's bundle by Package.swift
        let url = Bundle.module.url(forResource: "menu", withExtension: "json", subdirectory: "Default/screens")!
        return (try? JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: url))) ?? .null
    }()
}

extension Fixtures {
    internal static let menu = JSONValue(
        json: #"""
            {"type": "screen", "id": "menu", "blocks": [{"type": "account"},
              {"type": "group", "title": "Settings", "rows": [
                {"id": "look", "title": "Look", "symbol": "paintpalette.fill", "tone": "warn", "value": "Modern · Farm"},
                {"id": "notifications", "title": "Notifications", "symbol": "bell.badge.fill", "tone": "bad",
                 "value": "Asks, done"},
                {"id": "signins", "title": "Sign-ins", "symbol": "person.badge.key.fill", "tone": "purple",
                 "value": "Claude, Codex"},
                {"id": "devices", "title": "Devices", "symbol": "iphone.gen3", "tone": "info", "value": "3"},
                {"id": "terminal", "title": "Terminal", "symbol": "terminal.fill", "tone": "mute", "value": "mini"}]},
              {"type": "group", "rows": [
                {"id": "machines", "title": "Machines", "symbol": "desktopcomputer", "tone": "working",
                 "value": "mini, lab · Terminal"},
                {"id": "plugins", "title": "Plugins", "symbol": "puzzlepiece.extension.fill", "tone": "ok", "value": "4"}]},
              {"type": "note", "text": "Machines, Terminal and the plugin sheets open in v2.",
               "mono": "-- v2: these open their screens"}]}
            """#)
}
