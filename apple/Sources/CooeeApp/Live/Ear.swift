import AVFoundation
import CooeeFeatures
import Foundation
import Speech

@MainActor
internal final class Ear {
    private let engine = AVAudioEngine()
    private let analyzer: SpeechAnalyzer
    private let transcriber: DictationTranscriber
    private var input: AsyncStream<AnalyzerInput>.Continuation?

    private init(_ transcriber: DictationTranscriber) {
        self.transcriber = transcriber
        analyzer = SpeechAnalyzer(modules: [transcriber])
    }

    nonisolated internal static func dictation() -> AsyncThrowingStream<Heard, any Error> {
        results { continuation in
            var settled = ""
            return { result in
                let text = String(result.text.characters).trimmingCharacters(in: .whitespaces)
                let unsure = Unsure.between(text, result.alternatives.map { String($0.characters) })
                if result.isFinal { settled = [settled, text].filter { !$0.isEmpty }.joined(separator: " ") }
                continuation.yield(Heard(settled: settled, pending: result.isFinal ? "" : text, unsure: unsure))
            }
        }
    }

    nonisolated internal static func lines() -> AsyncThrowingStream<String, any Error> {
        results(call: true) { continuation in
            { result in
                let line = String(result.text.characters).trimmingCharacters(in: .whitespaces)
                if result.isFinal, !line.isEmpty { continuation.yield(line) }
            }
        }
    }

    internal static func session(call: Bool) throws {
        #if os(iOS)
            let session = AVAudioSession.sharedInstance()
            try session.setCategory(.playAndRecord, mode: call ? .voiceChat : .default, options: [.defaultToSpeaker])
            try session.setActive(true)
            if call { try session.overrideOutputAudioPort(.none) }
        #endif
    }

    internal static func speaker(_ loud: Bool) {
        #if os(iOS)
            try? AVAudioSession.sharedInstance().overrideOutputAudioPort(loud ? .speaker : .none)
        #endif
    }

    nonisolated private static func results<Out: Sendable>(
        call: Bool = false,
        _ reading:
            @escaping @MainActor (AsyncThrowingStream<Out, any Error>.Continuation)
            -> (DictationTranscriber.Result) -> Void
    ) -> AsyncThrowingStream<Out, any Error> {
        AsyncThrowingStream { continuation in
            let task = Task { @MainActor in
                do {
                    let ear = try await Ear.start(call: call)
                    let read = reading(continuation)
                    try await withTaskCancellationHandler {
                        for try await result in ear.transcriber.results { read(result) }
                    } onCancel: {
                        Task { @MainActor in await ear.stop() }
                    }
                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }

    private static func start(call: Bool) async throws -> Ear {
        guard await AVAudioApplication.requestRecordPermission() else { throw Refused.microphone }
        let allowed = await withCheckedContinuation { done in
            SFSpeechRecognizer.requestAuthorization { @Sendable in done.resume(returning: $0) }
        }
        guard allowed == .authorized else { throw Refused.speech }
        try session(call: call)
        let ear = Ear(
            DictationTranscriber(
                locale: try await locale(), contentHints: [], transcriptionOptions: [],
                reportingOptions: call ? [] : [.volatileResults, .alternativeTranscriptions], attributeOptions: []))
        try await ear.listen()
        return ear
    }

    private static func locale() async throws -> Locale {
        for language in Locale.preferredLanguages {
            if let locale = await DictationTranscriber.supportedLocale(equivalentTo: Locale(identifier: language)) {
                return locale
            }
        }
        throw Refused.language
    }

    private func listen() async throws {
        if let install = try await AssetInventory.assetInstallationRequest(supporting: [transcriber]) {
            try await install.downloadAndInstall()
        }
        guard let format = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [transcriber]) else {
            throw Refused.language
        }
        let (stream, input) = AsyncStream.makeStream(of: AnalyzerInput.self)
        self.input = input
        try await analyzer.start(inputSequence: stream)
        Ear.feed(engine.inputNode, to: format, into: input)
        engine.prepare()
        try engine.start()
    }

    internal static func feed(
        _ node: AVAudioNode, to format: AVAudioFormat, into input: AsyncStream<AnalyzerInput>.Continuation
    ) {
        let source = node.outputFormat(forBus: 0)
        let resample = Resample(from: source, to: format)
        node.installTap(onBus: 0, bufferSize: 4096, format: source) { @Sendable buffer, _ in
            if let converted = resample?.convert(buffer) { input.yield(AnalyzerInput(buffer: converted)) }
        }
    }

    private func stop() async {
        engine.inputNode.removeTap(onBus: 0)
        engine.stop()
        input?.finish()
        try? await analyzer.finalizeAndFinishThroughEndOfInput()
    }
}

private enum Refused: LocalizedError {
    case microphone, speech, language

    fileprivate var errorDescription: String? {
        switch self {
        case .microphone: "cooee can't use the microphone; allow it in Settings."
        case .speech: "cooee can't recognize speech; allow Speech Recognition in Settings."
        case .language: "Dictation doesn't speak your phone's languages here."
        }
    }
}

private final class Resample: @unchecked Sendable {
    private let converter: AVAudioConverter
    private let format: AVAudioFormat

    fileprivate init?(from source: AVAudioFormat, to format: AVAudioFormat) {
        guard let converter = AVAudioConverter(from: source, to: format) else { return nil }
        self.converter = converter
        self.format = format
    }

    fileprivate func convert(_ buffer: AVAudioPCMBuffer) -> AVAudioPCMBuffer? {
        let frames = AVAudioFrameCount(Double(buffer.frameLength) * format.sampleRate / buffer.format.sampleRate) + 1
        guard let out = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: frames) else { return nil }
        var fed = false
        converter.convert(to: out, error: nil) { _, status in
            defer { fed = true }
            status.pointee = fed ? .noDataNow : .haveData
            return fed ? nil : buffer
        }
        return out
    }
}
