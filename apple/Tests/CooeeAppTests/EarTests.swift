import AVFoundation
@testable import CooeeApp
import Speech
import Testing

@MainActor
internal struct EarTests {
    @Test internal func theMicrophoneTapFeedsTheAnalyzerFromTheAudioThread() async throws {
        let source = try #require(AVAudioFormat(standardFormatWithSampleRate: 48_000, channels: 1))
        let wanted = try #require(
            AVAudioFormat(commonFormat: .pcmFormatInt16, sampleRate: 16_000, channels: 1, interleaved: false))
        let engine = AVAudioEngine()
        try engine.enableManualRenderingMode(.offline, format: source, maximumFrameCount: 4096)
        let player = AVAudioPlayerNode()
        engine.attach(player)
        engine.connect(player, to: engine.mainMixerNode, format: source)
        let (stream, input) = AsyncStream.makeStream(of: AnalyzerInput.self)
        Ear.feed(engine.mainMixerNode, to: wanted, into: input)
        try engine.start()
        player.play()
        let render = Task.detached {
            let out = AVAudioPCMBuffer(pcmFormat: engine.manualRenderingFormat, frameCapacity: 4096)
            for _ in 0..<3 { if let out { _ = try engine.renderOffline(4096, to: out) } }
        }
        var heard = stream.makeAsyncIterator()
        let first = try #require(await heard.next())
        try await render.value
        engine.stop()
        #expect(first.buffer.format == wanted)
        #expect(first.buffer.frameLength > 0)
    }
}
