import CooeeCore
import SwiftUI

internal struct InstalledPlugin: Identifiable {
    internal let id: String
    internal let version: String
    internal let commit: String
    internal var listed = true
    internal let state: String
    internal var enabled = true
    internal var update = ""
    internal var personas: Set<String> = []
}

extension InstalledPlugin {
    internal static let fixtures: [InstalledPlugin] = [
        InstalledPlugin(
            id: "queue", version: "0.4.1", commit: "a1b2c3d", state: "running", personas: ["cond"]),
        InstalledPlugin(
            id: "events", version: "0.2.0", commit: "9f8e7d6", state: "running", update: "0.3.0",
            personas: ["cond", "chief"]),
        InstalledPlugin(
            id: "crm-sync", version: "1.0.2", commit: "44e1a0b", listed: false, state: "stopped", enabled: false),
    ]

    internal static let index: [(name: String, about: String)] = [
        ("calendar", "Today's meetings as cards; asks before it books"),
        ("inbox", "Mail threads a persona may read and draft"),
    ]
}

public struct PluginsView: View {
    @Environment(\.look) private var look
    private let plugins = InstalledPlugin.fixtures
    private let phone: Bool

    public var body: some View {
        SettingsPage(title: "Plugins", phone: phone) {
            Group2(header: "Installed") {
                ForEach(plugins) { plugin in
                    PluginLine(plugin: plugin, last: plugin.id == plugins.last?.id)
                }
            }
            Group2(header: "Add from the index") {
                ForEach(InstalledPlugin.index, id: \.name) { entry in
                    Line {
                        HStack(spacing: 10) {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(entry.name).ui(16, .medium, look)
                                Text(entry.about).ui(12, .regular, look).foregroundStyle(look.palette.mute)
                            }
                            Spacer(minLength: 6)
                            ActionPill(title: "Add…")
                        }
                    }
                }
                Line(divider: false) {
                    Text(look.mono ? "+ git url" : "Add from a git URL…").ui(15, .regular, look)
                        .foregroundStyle(look.palette.link)
                }
            }
        }
    }

    public init(phone: Bool = true) {
        self.phone = phone
    }
}

internal struct PluginLine: View {
    @Environment(\.look) private var look
    internal let plugin: InstalledPlugin
    internal let last: Bool

    internal var body: some View {
        let palette = look.palette
        Line(divider: !last) {
            HStack(spacing: 8) {
                Text(plugin.id).ui(16, .semibold, look)
                Text("\(plugin.version) · \(plugin.commit)").ui(12, .regular, look).monospaced()
                    .foregroundStyle(palette.mute)
                Spacer(minLength: 6)
                OnOff(active: plugin.enabled)
            }
            HStack(spacing: 8) {
                StateWord(text: plugin.state, tint: plugin.enabled ? palette.green : palette.mute)
                if !plugin.listed { ActionPill(title: "Not reviewed", tint: palette.amber) }
                if !plugin.update.isEmpty { ActionPill(title: "Update to \(plugin.update)", primary: true) }
            }
            personas
            HStack(spacing: 14) {
                if plugin.update.isEmpty { action("Update", palette.link) }
                action("Logs", palette.link)
                action("Remove", palette.red)
            }
        }
    }

    private var personas: some View {
        HStack(spacing: 6) {
            ForEach(Fixtures.personas) { persona in
                let active = plugin.personas.contains(persona.id)
                Avatar(persona: persona, size: 26)
                    .overlay(alignment: .bottomTrailing) {
                        if active && !look.mono {
                            Image(systemName: "checkmark.circle.fill").font(.system(size: 11))
                                .foregroundStyle(look.palette.green).background(look.palette.card, in: Circle())
                        }
                    }
                    .opacity(active ? 1 : 0.3)
            }
            Text(names).ui(12, .regular, look).foregroundStyle(look.palette.mute).lineLimit(1)
        }
    }

    private var names: String {
        let active = Fixtures.personas.filter { plugin.personas.contains($0.id) }.map(\.api.name)
        return active.isEmpty ? "no persona" : active.joined(separator: ", ")
    }

    private func action(_ title: String, _ tint: Color) -> some View {
        Text(look.mono ? "[\(title.lowercased())]" : title).ui(13, .medium, look).foregroundStyle(tint)
    }
}

internal struct OnOff: View {
    @Environment(\.look) private var look
    internal let active: Bool

    internal var body: some View {
        let palette = look.palette
        if look.mono {
            Text(active ? "[on]" : "[off]").ui(13, .semibold, look).foregroundStyle(
                active ? palette.green : palette.mute)
        } else {
            Capsule().fill(active ? palette.green : palette.edge).frame(width: 46, height: 28)
                .overlay(alignment: active ? .trailing : .leading) {
                    Circle().fill(.white).padding(2.5).shadow(color: .black.opacity(0.2), radius: 2, y: 1)
                }
        }
    }
}
