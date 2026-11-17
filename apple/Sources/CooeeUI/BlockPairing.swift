import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import SwiftUI

/// `core:/screens/pairing`: the step the pairing reducer is on, drawn from the bundled screen's blocks.
public struct PairingBlocks: View {
    @Environment(\.look) private var look
    private let fixed: PhoneStep
    private let store: StoreOf<PairingFeature>?

    private var step: PhoneStep { store?.step ?? fixed }

    private var values: [String: String] {
        guard let store else {
            let found = "mini · cooee 0.9"
            return [
                "host": "mini", "word": "maple", "by": "Guy", "found": found, "left": "4:51",
                "count": "\(Fixtures.personas.count)",
            ]
        }
        var values = ["host": store.host, "word": store.word, "count": "\(store.personas.count)", "left": "{left}"]
        values["error"] = store.error
        return values
    }

    public var body: some View {
        let screen = JSONValue.bundledPairing.list("blocks").first { $0.id == step.rawValue }
        ZStack {
            Backdrop()
            VStack(spacing: screen?["spacing"]?.number.map { CGFloat($0) } ?? 18) {
                ForEach(shown(screen?.list("blocks") ?? [])) { item in
                    PairingBlock(block: item.block, store: store)
                        .padding(.top, item.block["top"]?.number.map { CGFloat($0) } ?? 0)
                }
            }
            .padding(.horizontal, 24).padding(.bottom, 30)
        }
        .foregroundStyle(look.palette.foreground)
        .environment(\.stepBack, { store?.send(.show(.welcome)) })
    }

    public init(step: PhoneStep) {
        fixed = step
        store = nil
    }

    public init(store: StoreOf<PairingFeature>) {
        fixed = .welcome
        self.store = store
    }

    private func shown(_ blocks: [JSONValue]) -> [IndexedBlock] {
        blocks.enumerated().compactMap { index, block in
            guard let text = block.string("text") else { return IndexedBlock(id: index, block: block) }
            guard let filled = Self.filled(text, values) else { return nil }
            return IndexedBlock(id: index, block: block.setting("text", to: filled))
        }
    }

    internal static func filled(_ template: String, _ values: [String: String]) -> String? {
        let keys = template.matches(of: #/\{(\w+)\}/#).map { String($0.1) }
        guard keys.allSatisfy({ values[$0] != nil }) else { return nil }
        return keys.reduce(template) { $0.replacingOccurrences(of: "{\($1)}", with: values[$1] ?? "") }
    }
}

private struct IndexedBlock: Identifiable {
    fileprivate let id: Int
    fileprivate let block: JSONValue
}

extension JSONValue {
    internal static let bundledPairing: JSONValue = {
        // SAFETY: Default/screens/pairing.json is copied into this module's bundle by Package.swift
        let url = Bundle.module.url(forResource: "pairing", withExtension: "json", subdirectory: "Default/screens")!
        // SAFETY: the pairing snapshot frames draw every step of the bundled file, so a bad edit fails them first
        return try! JSONDecoder().decode(JSONValue.self, from: Data(contentsOf: url))
    }()

    fileprivate func setting(_ key: String, to text: String) -> Self {
        var fields = fields
        fields[key] = .string(text)
        return .object(fields)
    }
}

private struct PairingBlock: View {
    @Environment(\.look) private var look
    fileprivate let block: JSONValue
    fileprivate let store: StoreOf<PairingFeature>?

    fileprivate var body: some View {
        switch block.type {
        case "spacer": Spacer()
        case "mark": AppMark(size: size(120))
        case "text": text
        case "symbol":
            Image(systemName: block.string("symbol") ?? "").font(.system(size: size(17))).foregroundStyle(tone)
        case "found":
            Label(block.string("text") ?? "", systemImage: "checkmark.circle.fill").ui(15, .semibold, look)
                .foregroundStyle(look.palette.green)
        case "word": word
        default: PairingPart(block: block, store: store)
        }
    }

    private var text: some View {
        let text = Text(Markdown.inline(block.string("text") ?? ""))
        let weight = Self.weights[block.string("weight") ?? ""] ?? .regular
        return Group {
            if block.string("font") == "rounded" {
                text.font(.system(size: size(17), weight: weight, design: .rounded))
            } else {
                text.ui(size(17), weight, look)
            }
        }
        .foregroundStyle(tone).multilineTextAlignment(block["center"]?.bool == true ? .center : .leading)
    }

    private var word: some View {
        Text(block.string("text") ?? "").font(.system(size: 44, weight: .bold, design: .monospaced)).tracking(4)
            .foregroundStyle(look.palette.lead)
            .padding(.horizontal, 28).padding(.vertical, 12).surface(look, radius: 22)
    }

    private var tone: Color { look.palette.tone(block.string("tone")) ?? look.palette.foreground }

    private static let weights: [String: Font.Weight] = ["medium": .medium, "semibold": .semibold, "bold": .bold]

    private func size(_ fallback: CGFloat) -> CGFloat { block["size"]?.number.map { CGFloat($0) } ?? fallback }
}
