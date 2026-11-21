import Blocks
import BlocksUI
import CooeeCore
import SwiftUI

/// A screen of blocks drawn by the blocks package in cooee's look; cooee's own stage draws its scene.
public struct BlockScreen: View {
    /// The padding a host puts around the blocks; a full-bleed block cancels it.
    public static let inset = BlocksUI.BlocksView.inset

    // SAFETY: `scene` is the one core kind AppBlocks lets an app draw, so this table never clashes
    private static let kinds = try! AppBlocks(["scene": { AnyView(LaneStage(block: $0)) }], unknown: .nothing)

    @Environment(\.look) private var look: Look
    @Environment(\.act) private var acts
    @Environment(\.openThread) private var opensThread
    private let block: JSONValue
    private let source: CooeeCore.LayoutSource?
    private let focus: String?
    private let picked: String?
    private let beside: JSONValue?
    private var act: (@MainActor (BlockAction) -> Void)?
    private var open: @MainActor (String) -> Void = { _ in }
    private var openThread: (@MainActor (String) -> Void)?
    private var refresh: (@MainActor () -> Void)?

    public init(
        _ block: JSONValue, source: CooeeCore.LayoutSource?, focus: String? = nil, picked: String? = nil,
        beside: JSONValue? = nil, refresh: (@MainActor () -> Void)? = nil,
        act: (@MainActor (BlockAction) -> Void)? = nil, open: (@MainActor (String) -> Void)? = nil,
        openThread: (@MainActor (String) -> Void)? = nil
    ) {
        (self.block, self.source, self.focus, self.picked, self.beside) = (block, source, focus, picked, beside)
        (self.refresh, self.act, self.openThread) = (refresh, act, openThread)
        if let open { self.open = open }
    }

    public var body: some View {
        BlocksUI.BlocksView(
            block.typed, source: source.map(\.blocks), focus: focus, picked: picked, beside: beside?.typed,
            act: act ?? acts, open: open, openThread: openThread ?? opensThread, refresh: refresh
        )
        .blocksLook(BlocksUI.Look(look))
        .appBlocks(Self.kinds)
    }
}

extension JSONValue {
    /// The block this JSON holds, typed as the blocks package draws it; one that does not fit is an app block.
    internal var typed: Blocks.Block { (try? decode(Blocks.Block.self)) ?? .appBlock(self) }
}

extension CooeeCore.LayoutSource {
    fileprivate var blocks: Blocks.LayoutSource { self == .core ? .core : .plugin(name) }
}

extension BlocksUI.Look {
    fileprivate init(_ look: Look) {
        let palette = look.palette
        self.init(
            palette: BlocksUI.Palette(
                background: palette.background, card: palette.card, edge: palette.edge, line: palette.line,
                foreground: palette.foreground, mute: palette.mute, lead: palette.lead, link: palette.link,
                green: palette.green, amber: palette.amber, red: palette.red, purple: palette.purple,
                cyan: palette.cyan, dark: palette.dark),
            style: (try? JSONValue(look.style).decode(Blocks.Style.self)) ?? .bundled, still: look.still, tiles: nil)
    }
}

extension Source {
    /// What a JSON act sends: its action, tap and values.
    internal init(_ json: JSONValue) {
        self.init(action: json.string("action"), tap: try? json["tap"]?.decode(Tap.self), values: json["values"]?.fields)
    }
}

private struct ActKey: EnvironmentKey {
    fileprivate static var defaultValue: @MainActor (BlockAction) -> Void { { _ in } }
}

private struct OpenThreadKey: EnvironmentKey {
    fileprivate static var defaultValue: @MainActor (String) -> Void { { _ in } }
}

extension EnvironmentValues {
    internal var act: @MainActor (BlockAction) -> Void {
        get { self[ActKey.self] }
        set { self[ActKey.self] = newValue }
    }

    internal var openThread: @MainActor (String) -> Void {
        get { self[OpenThreadKey.self] }
        set { self[OpenThreadKey.self] = newValue }
    }
}
