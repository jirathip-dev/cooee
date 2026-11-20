import CooeeCore
import Testing

internal struct LiveReplyTests {
    @Test internal func theReplySoFarParsesLikeTheWholeTextWithItsCaretHoweverItIsChunked() {
        let text = "Done:\n\n```ts\nawait x();\n\n```\n- one\n* two\n\n  \nafter\n\nlast"
        let whole: [Block] = [
            .text("Done:"), .code(lang: "ts", "await x();\n"), .bullets(["one", "two"]), .text("after"),
            .text("last ▍"),
        ]
        #expect(LiveReply(text).blocks == whole && LiveReply().isEmpty && LiveReply().blocks.isEmpty)
        for size in 1...text.count {
            var reply = LiveReply()
            var rest = Substring(text)
            while !rest.isEmpty {
                reply.append(String(rest.prefix(size)))
                rest = rest.dropFirst(size)
            }
            #expect(reply.blocks == whole && reply.text == text, "chunks of \(size)")
        }
    }

    @Test internal func aLongReplyInManyChunksParsesLinearlyInItsLength() {
        let paragraph = "A line of the reply with **some** words.\n- one bullet\n- two\n\n"
        let fence = "```swift\nlet x = 1\n\nlet y = 2\n```\n"
        var text = ""
        while text.count < 20_000 { text += text.count % 1_000 < 900 ? paragraph : fence }
        text = String(text.prefix(20_000))
        var parsed = 0
        var reply = LiveReply()
        for start in stride(from: 0, to: text.count, by: 50) {
            let chunk = String(text.dropFirst(start).prefix(50))
            reply.append(chunk) { markdown in
                parsed += markdown.count
                return Markdown.blocks(markdown)
            }
        }
        #expect(reply.blocks == Markdown.blocks(text + " ▍"))
        #expect(parsed < 3 * text.count, "parsed \(parsed) characters")
    }
}
