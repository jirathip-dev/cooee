import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct PersonaConnectors: View {
    @Environment(\.look) private var look
    @Binding internal var draft: APIPersona
    internal let list: ConnectorList
    internal var catalogue: () -> Void = {}

    @ViewBuilder internal var body: some View {
        if !draft.id.isEmpty, !list.connectors.isEmpty || draft.mcp?.isEmpty == false { group }
    }

    private var group: some View {
        Group2(header: "Connectors") {
            ForEach(draft.mcp ?? [], id: \.server) { pick in
                if let connector = list.connectors.first(where: { $0.name == pick.server }) {
                    PickLine(connector: connector, account: pick.account ?? "", pick: pickHere, catalogue: catalogue)
                }
            }
            if look.still {
                AttachRow()
            } else {
                Menu {
                    Section("Signed-in accounts") {
                        ForEach(Array(signedIn.enumerated()), id: \.offset) { _, pair in
                            let (connector, account) = pair
                            Button("\(connector.label) · \(account)") { draft.pick(connector.name, account: account) }
                                .disabled(attached(connector.name, account))
                        }
                    }
                    Button("From the catalogue…", action: catalogue)
                } label: {
                    AttachRow()
                }
                .buttonStyle(.plain)
            }
        }
    }

    private var signedIn: [(Connector, String)] {
        list.connectors.flatMap { connector in connector.signedIn.map { (connector, $0.name) } }
    }

    private func attached(_ server: String, _ account: String) -> Bool {
        draft.mcp?.contains { $0.server == server && $0.account == account } == true
    }

    private func pickHere(_ server: String, _ account: String?) { draft.pick(server, account: account) }
}

internal struct AttachRow: View {
    @Environment(\.look) private var look

    internal var body: some View {
        Line(divider: false) {
            HStack(spacing: 8) {
                Glyph(symbol: "plus.circle.fill", ascii: "+", size: 16)
                Text(look.quiet ? "attach a connector" : "Attach a connector").ui(15, .regular, look)
            }
            .foregroundStyle(look.palette.link)
        }
        .contentShape(.rect)
    }
}

internal struct PickLine: View {
    @Environment(\.look) private var look
    internal let connector: Connector
    internal let account: String
    internal var pick: (String, String?) -> Void = { _, _ in }
    internal var catalogue: () -> Void = {}

    internal var body: some View {
        let palette = look.palette
        let needs = connector.accounts.first { $0.name == account }?.state != .signedIn
        Line(inset: 62) {
            HStack(alignment: .center, spacing: 12) {
                ConnectorMark(connector: connector)
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 8) {
                        Text(connector.label).ui(16, .semibold, look)
                        chip
                    }
                    if needs {
                        Text("needs sign-in · its tools are off until then").ui(12, .medium, look)
                            .foregroundStyle(palette.amber)
                    } else {
                        ToolCounts(tools: connector.tools)
                    }
                }
                Spacer(minLength: 4)
                if needs {
                    Button(action: catalogue) { ActionPill(title: "Sign in", primary: true, tint: palette.amber) }
                        .buttonStyle(.plain)
                } else {
                    Button {
                        pick(connector.name, nil)
                    } label: {
                        ActionPill(title: "Detach", tint: palette.red)
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }

    @ViewBuilder private var chip: some View {
        if look.still {
            AccountChip(name: account)
        } else {
            Menu {
                ForEach(connector.signedIn) { other in
                    Button(other.name) { pick(connector.name, other.name) }
                }
            } label: {
                AccountChip(name: account)
            }
            .buttonStyle(.plain)
        }
    }
}

internal struct AccountChip: View {
    @Environment(\.look) private var look
    internal let name: String

    internal var body: some View {
        HStack(spacing: 3) {
            Text(look.textMarks ? "[\(name)" : name).ui(12, .medium, look)
            Glyph(symbol: "chevron.up.chevron.down", ascii: "v]", size: 9)
        }
        .foregroundStyle(look.palette.link).padding(.horizontal, look.outline ? 0 : 8).frame(height: 22)
        .background(look.outline ? Color.clear : look.palette.link.opacity(0.14), in: Capsule())
    }
}
