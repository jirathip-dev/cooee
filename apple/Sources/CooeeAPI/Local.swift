#if os(macOS)
    import CooeeCore
    import Foundation

    public enum Shell {
        /// `executable`'s standard output, run off the concurrency pool with no input, its output in files (a child it
        /// leaves running holds no pipe open) and killed after `limit` seconds; an `APIError` with its standard error
        /// when it fails.
        public static func run(_ executable: URL, _ arguments: [String], limit: Double = 60) async throws -> String {
            try await withCheckedThrowingContinuation { done in
                Thread { done.resume(with: Result { try wait(executable, arguments, limit: limit) }) }.start()
            }
        }

        private static func wait(_ executable: URL, _ arguments: [String], limit: Double) throws -> String {
            let files = FileManager.default.temporaryDirectory.appending(path: "cooee-run-\(UUID().uuidString)")
            let (out, err) = (files.appendingPathExtension("out"), files.appendingPathExtension("err"))
            FileManager.default.createFile(atPath: out.path(), contents: nil)
            FileManager.default.createFile(atPath: err.path(), contents: nil)
            defer { try? FileManager.default.removeItem(at: out) }
            defer { try? FileManager.default.removeItem(at: err) }
            let process = Process()
            process.executableURL = executable
            process.arguments = arguments
            process.standardInput = FileHandle.nullDevice
            process.standardOutput = try FileHandle(forWritingTo: out)
            process.standardError = try FileHandle(forWritingTo: err)
            guard try exited(process, within: limit) else {
                throw APIError(status: 0, text: "`\(executable.lastPathComponent)` took over \(Int(limit)) s.")
            }
            let error = (try? String(contentsOf: err, encoding: .utf8)) ?? ""
            guard process.terminationStatus == 0 else {
                throw APIError(status: Int(process.terminationStatus), text: error.trimmingCharacters(in: .newlines))
            }
            return (try? String(contentsOf: out, encoding: .utf8)) ?? ""
        }

        private static func exited(_ process: Process, within limit: Double) throws -> Bool {
            try process.run()
            let deadline = Date.now.addingTimeInterval(limit)
            while process.isRunning && Date.now < deadline { Thread.sleep(forTimeInterval: 0.02) }
            if process.isRunning { kill(process.processIdentifier, SIGKILL) }
            process.waitUntilExit()
            return process.terminationReason == .exit
        }
    }

    public struct LocalConfig: Sendable {
        public static let standard = LocalConfig(
            file: URL.homeDirectory.appending(path: ".config/cooee/cooee/config.toml"))

        public let file: URL

        public var port: Int { value("port").flatMap { Int($0) } ?? 4747 }

        public init(file: URL) {
            self.file = file
        }

        private func value(_ key: String) -> String? {
            let lines = contents()
            return lines[..<Self.top(lines)].first { Self.key($0) == key }.map { line in
                line.split(separator: "=", maxSplits: 1).last.map { $0.trimmingCharacters(in: .whitespaces) } ?? ""
            }
        }

        private static func top(_ lines: [String]) -> Int {
            lines.firstIndex { $0.trimmingCharacters(in: .whitespaces).hasPrefix("[") } ?? lines.count
        }

        private static func key(_ line: String) -> String? {
            let parts = line.split(separator: "=", maxSplits: 1)
            guard parts.count == 2 else { return nil }
            return parts[0].trimmingCharacters(in: .whitespaces)
        }

        private func contents() -> [String] {
            let text = (try? String(contentsOf: file, encoding: .utf8)) ?? ""
            let top = text.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
            return top.isEmpty || text.isEmpty ? [] : top
        }
    }
#endif
