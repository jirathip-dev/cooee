#if os(macOS)
    import AppKit
    import CooeeCore
    import CooeeUI
    import SwiftUI
    import Testing

    @MainActor
    internal struct BlockIdentityTests {
        private static let ask = JSONValue.object([
            "type": .string("choice"), "id": .string("ask"), "action": .string("answer"),
            "options": .array([]), "other": .object(["placeholder": .string("why")]),
        ])

        private static let row = JSONValue.object([
            "type": .string("row"), "id": .string("new"), "title": .string("A new row"),
        ])

        private static func screen(_ blocks: [JSONValue]) -> BlocksView {
            let screen = JSONValue.object([
                "type": .string("screen"), "title": .string("Ask"), "blocks": .array(blocks),
            ])
            return BlocksView(screen, source: .plugin("edgar"))
        }

        private static func box(in view: NSView) -> NSTextField? {
            if let field = view as? NSTextField, field.isEditable { return field }
            return view.subviews.lazy.compactMap { box(in: $0) }.first
        }

        @Test internal func aFieldKeepsItsTextAndFocusWhenARowIsInsertedAboveIt() throws {
            let host = NSHostingView(rootView: Self.screen([Self.ask]))
            let window = NSWindow(
                contentRect: NSRect(x: 0, y: 0, width: 402, height: 600), styleMask: [.titled], backing: .buffered,
                defer: false)
            window.contentView = host
            host.layoutSubtreeIfNeeded()
            let typed = try #require(Self.box(in: host))
            #expect(window.makeFirstResponder(typed))
            typed.currentEditor()?.insertText("half typed")
            host.rootView = Self.screen([Self.row, Self.ask])
            host.layoutSubtreeIfNeeded()
            let kept = try #require(Self.box(in: host))
            #expect(kept.stringValue == "half typed")
            #expect((window.firstResponder as? NSText)?.delegate === kept)
        }
    }
#endif
