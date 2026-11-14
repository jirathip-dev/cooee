import CooeeCore
import SwiftUI

public struct ChatActions {
    public var draft: Binding<String>?
    public var files: [String]
    public var send: (() -> Void)?
    public var attach: ([Attachment]) -> Void
    public var detach: (Int) -> Void
    public var stop: () -> Void
    public var steer: (Int) -> Void
    public var unqueue: (Int) -> Void
    public var answer: (Int, BlockAction) -> Void
    public var answering: Set<Int>
    public var back: () -> Void
    public var retry: (Int) -> Void = { _ in }
    public var places: [String] = []
    public var counts: [Int?] = []
    public var new: (() -> Void)?
    public var open: (Int) -> Void = { _ in }
    public var attachable: [String] = []
    public var attachPlugin: (Int) -> Void = { _ in }
    public var persona: (() -> Void)?
    public var choices: PersonaChoices?
    public var agent: @MainActor @Sendable (String) -> Void = { _ in }
    public var model: (String, String) -> Void = { _, _ in }

    public init(
        draft: Binding<String>? = nil,
        files: [String] = [],
        send: (() -> Void)? = nil,
        attach: @escaping ([Attachment]) -> Void = { _ in },
        detach: @escaping (Int) -> Void = { _ in },
        stop: @escaping () -> Void = {},
        steer: @escaping (Int) -> Void = { _ in },
        unqueue: @escaping (Int) -> Void = { _ in },
        answer: @escaping (Int, BlockAction) -> Void = { _, _ in },
        answering: Set<Int> = [],
        back: @escaping () -> Void = {}
    ) {
        self.draft = draft
        self.files = files
        self.send = send
        self.attach = attach
        self.detach = detach
        self.stop = stop
        self.steer = steer
        self.unqueue = unqueue
        self.answer = answer
        self.answering = answering
        self.back = back
    }
}
