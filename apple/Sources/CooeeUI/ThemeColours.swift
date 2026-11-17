import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

internal struct ThemeColours: View {
    private static let groups = [
        ("Text", ["--fg", "--mute", "--l", "--link"]),
        (
            "Surfaces · figures",
            [
                "--bg", "--card", "--edge", "--line", "--scrim", "--p", "--c", "--fk-l", "--fk-c", "--fk-a", "--fk-g",
                "--fk-p",
            ]
        ),
    ]
    private static let names = [
        "--a": "working", "--r": "needs you · failed", "--g": "done", "--fg": "text", "--mute": "quiet text",
        "--bg": "background", "--card": "cards", "--l": "accent", "--link": "links", "--edge": "edges",
        "--line": "lines",
        "--p": "purple", "--c": "blue", "--fk-l": "figure 1", "--fk-c": "figure 2", "--fk-a": "figure 3",
        "--fk-g": "figure 4", "--fk-p": "figure 5",
    ]

    @Environment(\.look) private var look
    internal let store: StoreOf<ThemeEditorFeature>

    private var dark: Bool { store.colours ?? true }

    internal var body: some View {
        let report = store.reports[dark ? "dark" : "light"]
        Scroller {
            VStack(alignment: .leading, spacing: 18) {
                Scroller(axis: .horizontal) {
                    let mode = dark ? "dark" : "light"
                    let schemes = store.schemes.filter { $0.value.mode == mode }.sorted {
                        $0.value.label < $1.value.label
                    }
                    ThemeChips(values: schemes.map(\.value.label), fill: false) { label in start(label, schemes) }
                }
                .frame(height: 40)
                ThemePreview(state: store.state, chat: true, dark: dark)
                let fails = report?.fails.count ?? 0
                Group2(
                    header: "States · theme gate "
                        + (report == nil ? "checking" : fails == 0 ? "✓ passes" : "✗ \(fails) to fix")
                ) {
                    ForEach(["--a", "--r", "--g"], id: \.self) { key in row(key, report?.marks[key]) }
                }
                ForEach(Self.groups, id: \.0) { header, keys in
                    Group2(header: header) { ForEach(keys, id: \.self) { key in row(key, report?.marks[key]) } }
                }
            }
            .padding(16)
        }
        .background(look.palette.background).foregroundStyle(look.palette.foreground)
        .navigationTitle(dark ? "Dark colours" : "Light colours")
        .toolbar { Button("Done") { store.send(.colours(nil)) } }
    }

    private func start(_ label: String, _ schemes: [(key: String, value: ThemeScheme)]) {
        if let name = schemes.first(where: { $0.value.label == label })?.key { store.send(.start(name)) }
    }

    @ViewBuilder private func row(_ key: String, _ mark: GateMark?) -> some View {
        let palette = look.palette
        let hex = store.state.tokens(dark: dark)[key] ?? ""
        Button {
            store.send(.open(store.open == key ? nil : key))
        } label: {
            HStack(spacing: 12) {
                RoundedRectangle(cornerRadius: 8).fill(Color.hex(UInt32(hex.dropFirst(), radix: 16) ?? 0))
                    .frame(width: 28, height: 28).overlay { RoundedRectangle(cornerRadius: 8).stroke(palette.edge) }
                VStack(alignment: .leading, spacing: 2) {
                    Text(Self.names[key] ?? key).ui(16, .regular, look)
                    Text("\(key) · \(hex)").ui(13, .regular, look).foregroundStyle(palette.mute)
                }
                Spacer()
                if let mark {
                    Text(
                        String(format: "%.1f:1 ", mark.value) + (mark.passes ? "✓" : "✗ needs \(mark.min.formatted())")
                    )
                    .font(.system(size: 14, weight: .semibold, design: .monospaced))
                    .foregroundStyle(mark.passes ? palette.green : palette.amber)
                }
            }
            .padding(.horizontal, 14).padding(.vertical, 8).contentShape(.rect)
        }
        .buttonStyle(.plain)
        if store.open == key, let shade = Shade(hex: hex) { fixer(key, shade, mark) }
    }

    private func fixer(_ key: String, _ shade: Shade, _ mark: GateMark?) -> some View {
        let set = { (change: (inout Shade) -> Void) in
            var shade = shade
            change(&shade)
            store.send(.token(key, shade.hex))
        }
        return VStack(alignment: .leading, spacing: 10) {
            if let mark {
                Text("\(mark.what): \(String(format: "%.2f", mark.value)):1, the gate needs \(mark.min.formatted()):1.")
                    .ui(14, .regular, look).foregroundStyle(look.palette.mute)
            }
            slider("Hue", Binding(get: { shade.hue }, set: { hue in set { $0.hue = hue } }))
            slider("Light", Binding(get: { shade.light }, set: { light in set { $0.light = light } }))
            if let fix = mark?.fix {
                let ratio = mark?.fixed.map { String(format: " · %.1f:1", $0) } ?? ""
                Button {
                    store.send(.token(key, fix))
                } label: {
                    Text((Shade(hex: fix)?.light ?? 0 < shade.light ? "Darken" : "Lighten") + " to \(fix)\(ratio) ✓")
                        .ui(15, .semibold, look).padding(.horizontal, 16).frame(height: 40)
                        .background(look.palette.lead, in: .capsule).foregroundStyle(look.palette.background)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 14).padding(.bottom, 12)
    }

    private func slider(_ title: String, _ value: Binding<Double>) -> some View {
        HStack(spacing: 12) {
            Text(title).ui(14, .regular, look).foregroundStyle(look.palette.mute).frame(width: 44, alignment: .leading)
            Slider(value: value).accessibilityLabel(title)
        }
    }
}
