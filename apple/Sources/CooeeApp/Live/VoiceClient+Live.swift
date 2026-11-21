import AVFoundation
import ComposableArchitecture
import CooeeCore
import CooeeFeatures
import Foundation
import Speech

extension VoiceClient: DependencyKey {
    public static let liveValue = VoiceClient(
        dictate: { Ear.dictation() },
        hear: { Ear.lines() },
        record: { try await Notes.shared.start() },
        recorded: { try await Notes.shared.stop() },
        speak: { await Mouth.shared.say($0) },
        hush: { await Mouth.shared.hush() },
        speaker: { await Ear.speaker($0) },
        calls: { await CallRequests.stream() }
    )
}

/// Calls asked for from outside the app (Siri, Shortcuts): a persona's id or name, held until the app listens.
@MainActor
public enum CallRequests {
    private static var listening: AsyncStream<String>.Continuation?
    private static var held: [String] = []

    public static func request(_ persona: String) {
        if let listening { listening.yield(persona) } else { held.append(persona) }
    }

    internal static func stream() -> AsyncStream<String> {
        let (stream, continuation) = AsyncStream.makeStream(of: String.self)
        listening = continuation
        for persona in held { continuation.yield(persona) }
        held = []
        return stream
    }
}

@MainActor
private final class Notes {
    fileprivate static let shared = Notes()
    private var recorder: AVAudioRecorder?

    fileprivate func start() throws {
        try Ear.session(call: false)
        let file = URL.temporaryDirectory.appending(path: "voice-note-\(UUID().uuidString).m4a")
        let settings: [String: Int] = [
            AVFormatIDKey: Int(kAudioFormatMPEG4AAC), AVSampleRateKey: 44_100, AVNumberOfChannelsKey: 1,
        ]
        let recorder = try AVAudioRecorder(url: file, settings: settings)
        recorder.record()
        self.recorder = recorder
    }

    fileprivate func stop() throws -> Attachment {
        guard let recorder else { throw CocoaError(.fileNoSuchFile) }
        recorder.stop()
        self.recorder = nil
        defer { try? FileManager.default.removeItem(at: recorder.url) }
        return Attachment(name: "voice-note.m4a", bytes: try Data(contentsOf: recorder.url))
    }
}

@MainActor
private final class Mouth: NSObject, AVSpeechSynthesizerDelegate {
    fileprivate static let shared = Mouth()
    private let synthesizer = AVSpeechSynthesizer()
    private var done: CheckedContinuation<Void, Never>?

    override private init() {
        super.init()
        synthesizer.delegate = self
    }

    fileprivate func say(_ text: String) async {
        let utterance = AVSpeechUtterance(string: text)
        let thai = text.unicodeScalars.contains { (0x0E00...0x0E7F).contains($0.value) }
        utterance.voice = AVSpeechSynthesisVoice(language: thai ? "th-TH" : nil)
        await withCheckedContinuation { continuation in
            finish()
            done = continuation
            synthesizer.speak(utterance)
        }
    }

    fileprivate func hush() {
        synthesizer.stopSpeaking(at: .immediate)
        finish()
    }

    private func finish() {
        done?.resume()
        done = nil
    }

    nonisolated fileprivate func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didFinish _: AVSpeechUtterance) {
        Task { @MainActor in Mouth.shared.finish() }
    }

    nonisolated fileprivate func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didCancel _: AVSpeechUtterance) {
        Task { @MainActor in Mouth.shared.finish() }
    }
}
