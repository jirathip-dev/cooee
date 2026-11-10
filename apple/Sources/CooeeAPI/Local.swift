#if os(macOS)
    import CooeeCore
    import Foundation

    public enum Shell {
        private static let agentsScript = #"""
            for a in claude codex hermes; do
              p=$(command -v "$a") && printf '%s\t%s\t%s\n' "$a" "$p" "$("$p" --version 2>/dev/null | head -1)"
            done
            true
            """#

        /// `executable`'s standard output, run off the concurrency pool with no input and stopped after `limit`
        /// seconds; an `APIError` with its standard error when it fails.
        public static func run(_ executable: URL, _ arguments: [String], limit: Double = 60) async throws -> String {
            try await withCheckedThrowingContinuation { done in
                DispatchQueue.global().async {
                    done.resume(with: Result { try wait(executable, arguments, limit: limit) })
                }
            }
        }

        private static func wait(_ executable: URL, _ arguments: [String], limit: Double) throws -> String {
            let process = Process()
            let (out, err) = (Pipe(), Pipe())
            process.executableURL = executable
            process.arguments = arguments
            process.standardInput = FileHandle.nullDevice
            process.standardOutput = out
            process.standardError = err
            try process.run()
            let pid = process.processIdentifier
            let stop = DispatchWorkItem { kill(pid, SIGTERM) }
            DispatchQueue.global().asyncAfter(deadline: .now() + limit, execute: stop)
            let output = String(bytes: out.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
            let error = String(bytes: err.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
            process.waitUntilExit()
            stop.cancel()
            guard process.terminationReason == .exit else {
                throw APIError(status: 0, text: "`\(executable.lastPathComponent)` took over \(Int(limit)) s.")
            }
            guard process.terminationStatus == 0 else {
                throw APIError(status: Int(process.terminationStatus), text: error.trimmingCharacters(in: .newlines))
            }
            return output
        }

        /// `script` in a login shell, so it sees the user's `PATH` as Terminal does.
        public static func login(_ script: String) async throws -> String {
            try await run(URL(filePath: "/bin/zsh"), ["-lc", script])
        }

        public static func agents() async -> [FoundAgent] {
            let home = URL.homeDirectory.path(percentEncoded: false)
            let prefix = home.hasSuffix("/") ? home : home + "/"
            let found = ((try? await login(agentsScript)) ?? "").replacingOccurrences(of: "\t\(prefix)", with: "\t~/")
            return FoundAgent.list(found: found)
        }

        /// This Mac's Bonjour name (`scutil`, which asks no one on the network) and its tailnet name, if any.
        public static func network() async -> Network {
            let named = try? await run(URL(filePath: "/usr/sbin/scutil"), ["--get", "LocalHostName"])
            let name = named?.trimmingCharacters(in: .whitespacesAndNewlines) ?? "this Mac"
            let status = (try? await login("tailscale status --json")) ?? ""
            return Network(name: name, lan: "\(name).local", tailnet: Network.tailnet(status: Data(status.utf8)))
        }
    }

    public struct LocalServer: Sendable {
        public let cooee: URL

        public init(cooee: URL) {
            self.cooee = cooee
        }

        public func code() async throws -> PairCode {
            let output = try await Shell.run(cooee, ["pair"])
            guard let code = PairCode(output: output) else {
                throw APIError(status: 0, text: "`cooee pair` printed no link.")
            }
            return code
        }

        public func requests() async throws -> [PairRequest] {
            try JSONDecoder().decode([PairRequest].self, from: Data(await Shell.run(cooee, ["pair", "asks"]).utf8))
        }

        public func answer(_ id: String, yes: Bool) async throws {
            _ = try await Shell.run(cooee, ["pair", yes ? "approve" : "decline", id])
        }

        public func signIns() async throws -> [SignIn] {
            let listed = try await Shell.run(cooee, ["accounts", "ls", "--json"])
            return try JSONDecoder().decode([SignIn].self, from: Data(listed.utf8))
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

        public func strings(_ key: String) -> [String] {
            value(key).flatMap { try? JSONDecoder().decode([String].self, from: Data($0.utf8)) } ?? []
        }

        public func value(_ key: String) -> String? {
            let lines = contents()
            return lines[..<Self.top(lines)].first { Self.key($0) == key }.map { line in
                line.split(separator: "=", maxSplits: 1).last.map { $0.trimmingCharacters(in: .whitespaces) } ?? ""
            }
        }

        /// Sets the top-level `key` to `value` (written as JSON, which TOML reads): replaced in place, else added first.
        public func set(_ key: String, _ value: some Encodable) throws {
            let encoder = JSONEncoder()
            encoder.outputFormatting = .withoutEscapingSlashes
            let line = "\(key) = \(String(bytes: try encoder.encode(value), encoding: .utf8) ?? "")"
            var lines = contents()
            if let index = lines[..<Self.top(lines)].firstIndex(where: { Self.key($0) == key }) {
                lines[index] = line
            } else {
                lines.insert(line, at: 0)
            }
            try FileManager.default.createDirectory(
                at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
            try lines.joined(separator: "\n").appending(lines.last == "" ? "" : "\n").write(
                to: file, atomically: true, encoding: .utf8)
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
