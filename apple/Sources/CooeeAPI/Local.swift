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

        /// `executable`'s standard output; an `APIError` with its standard error when it exits non-zero.
        public static func run(_ executable: URL, _ arguments: [String]) throws -> String {
            let process = Process()
            let (out, err) = (Pipe(), Pipe())
            process.executableURL = executable
            process.arguments = arguments
            process.standardOutput = out
            process.standardError = err
            try process.run()
            let output = String(bytes: out.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
            let error = String(bytes: err.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
            process.waitUntilExit()
            guard process.terminationStatus == 0 else {
                throw APIError(status: Int(process.terminationStatus), text: error.trimmingCharacters(in: .newlines))
            }
            return output
        }

        /// `script` in a login shell, so it sees the user's `PATH` as Terminal does.
        public static func login(_ script: String) throws -> String {
            try run(URL(filePath: "/bin/zsh"), ["-lc", script])
        }

        public static func agents() -> [FoundAgent] {
            let home = URL.homeDirectory.path(percentEncoded: false)
            let prefix = home.hasSuffix("/") ? home : home + "/"
            let found = ((try? login(agentsScript)) ?? "").replacingOccurrences(of: "\t\(prefix)", with: "\t~/")
            return FoundAgent.list(found: found)
        }

        public static func network() -> Network {
            let lan = ProcessInfo.processInfo.hostName
            let name = lan.split(separator: ".").first.map(String.init) ?? lan
            let status = (try? login("tailscale status --json")) ?? ""
            return Network(name: name, lan: lan, tailnet: Network.tailnet(status: Data(status.utf8)))
        }
    }

    public struct LocalServer: Sendable {
        public let cooee: URL

        public init(cooee: URL) {
            self.cooee = cooee
        }

        public func code() throws -> PairCode {
            let output = try Shell.run(cooee, ["pair"])
            guard let code = PairCode(output: output) else {
                throw APIError(status: 0, text: "`cooee pair` printed no link.")
            }
            return code
        }

        public func requests() throws -> [PairRequest] {
            try JSONDecoder().decode([PairRequest].self, from: Data(Shell.run(cooee, ["pair", "asks"]).utf8))
        }

        public func answer(_ id: String, yes: Bool) throws {
            _ = try Shell.run(cooee, ["pair", yes ? "approve" : "decline", id])
        }

        public func signIns() throws -> [SignIn] {
            try JSONDecoder().decode([SignIn].self, from: Data(Shell.run(cooee, ["accounts", "ls", "--json"]).utf8))
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
