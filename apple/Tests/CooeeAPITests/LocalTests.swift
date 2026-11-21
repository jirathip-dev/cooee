#if os(macOS)
    import CooeeAPI
    import CooeeCore
    import Foundation
    import Testing

    internal struct LocalTests {
        @Test internal func aCommandThatHangsIsStopped() async throws {
            await #expect(throws: APIError(status: 0, text: "`sleep` took over 1 s.")) {
                try await Shell.run(URL(filePath: "/bin/sleep"), ["30"], limit: 1)
            }
        }

        @Test internal func aCommandReturnsItsOutputOrFailsWithItsError() async throws {
            #expect(try await Shell.run(URL(filePath: "/bin/echo"), ["ok"]) == "ok\n")
            await #expect(throws: APIError(status: 3, text: "no")) {
                try await Shell.run(URL(filePath: "/bin/sh"), ["-c", "echo no >&2; exit 3"])
            }
        }

        @Test internal func thePortIsTheConfigsTopLevelOne() throws {
            let dir = FileManager.default.temporaryDirectory.appending(path: "cooee-local-\(UUID().uuidString)")
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            defer { try? FileManager.default.removeItem(at: dir) }
            #expect(LocalConfig(file: dir.appending(path: "none.toml")).port == 4747)
            let config = LocalConfig(file: dir.appending(path: "config.toml"))
            try "roots = [\"~/Projects\"]\nport = 4800\n\n[[plugins]]\nport = 5800\n".write(
                to: config.file, atomically: true, encoding: .utf8)
            #expect(config.port == 4800)
            #expect(LocalConfig.standard.file.path().hasSuffix("/.config/cooee/cooee/config.toml"))
        }
    }
#endif
