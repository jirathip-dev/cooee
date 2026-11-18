import CooeeCore
@testable import CooeeUI
import SwiftUI

public struct MacConnectors: View {
    @Environment(\.look) private var look

    private static let rows: [(String, String, KeyPath<Palette, Color>)] = [
        ("paintpalette.fill", "Look", \.amber), ("bell.badge.fill", "Notifications", \.red),
        ("person.badge.key.fill", "Sign-ins", \.purple), ("iphone.gen3", "Devices", \.link),
        ("terminal.fill", "Terminal", \.mute), ("desktopcomputer", "Machines", \.cyan),
        ("puzzlepiece.extension.fill", "Plugins", \.green),
        ("point.3.connected.trianglepath.dotted", "Connectors", \.purple),
    ]

    public var body: some View {
        let palette = look.palette
        MacChrome {
            HStack(spacing: 0) {
                sidebar
                VStack(alignment: .leading, spacing: 0) {
                    Text(look.mono ? "# connectors" : "Connectors").ui(26, .bold, look)
                        .padding(.horizontal, 24).padding(.top, 24).padding(.bottom, 8)
                    Scroller {
                        VStack(alignment: .leading, spacing: 22) { CatalogueGroups(selected: "linear") }
                            .padding(.horizontal, 16).padding(.vertical, 10)
                    }
                }
                .frame(width: 440)
                .overlay(alignment: .trailing) { Rectangle().fill(palette.line).frame(width: 1) }
                Scroller {
                    EntryBody(connector: Story.linear).frame(maxWidth: 620).padding(.horizontal, 32)
                        .padding(.vertical, 30).frame(maxWidth: .infinity)
                }
            }
            .foregroundStyle(palette.foreground)
        }
        .background(Backdrop())
    }

    private var sidebar: some View {
        let palette = look.palette
        return VStack(alignment: .leading, spacing: 2) {
            Text(look.mono ? "# settings" : "Settings").ui(13, .semibold, look).foregroundStyle(palette.mute)
                .padding(.leading, 10).padding(.bottom, 6)
            ForEach(Self.rows, id: \.1) { row in
                let active = row.1 == "Connectors"
                HStack(spacing: 10) {
                    if !look.mono {
                        Image(systemName: row.0).font(.system(size: 11, weight: .semibold)).foregroundStyle(.white)
                            .frame(width: 22, height: 22).background(palette[keyPath: row.2], in: .rect(cornerRadius: 6))
                    }
                    Text(look.mono && active ? "> \(row.1.lowercased())" : row.1).ui(14, active ? .semibold : .regular, look)
                    Spacer()
                    if active { Text("●").font(.system(size: 8)).foregroundStyle(palette.amber) }
                }
                .padding(.horizontal, 10).frame(height: 34)
                .background(active ? palette.link.opacity(0.18) : .clear, in: .rect(cornerRadius: look.mono ? 0 : 8))
            }
            Spacer()
        }
        .padding(.top, 48).padding(.horizontal, 10).frame(width: 230)
        .background(palette.card.opacity(look.style.surface == .glass ? 0.45 : 0.8))
        .overlay(alignment: .trailing) { Rectangle().fill(palette.line).frame(width: 1) }
    }

    public init() {}
}
