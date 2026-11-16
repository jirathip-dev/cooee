#if os(macOS)
    import ComposableArchitecture
    import CooeeAPI
    import CooeeCore
    import CooeeFeatures
    import Foundation
    import LocalAuthentication
    import ServiceManagement

    internal enum LoginItem: String, CaseIterable {
        case cooee = "com.jirathip.cooee.serve"
        case shed = "com.jirathip.cooee.shed"

        private static let restartScript = #"""
            lock="${TMPDIR:-/tmp}/cooee-update.lock"
            mkdir "$lock" 2>/dev/null || exit 0
            trap 'rmdir "$lock"' EXIT
            echo "$(date) waiting for $1 to leave version $2"
            while [ "$(/usr/bin/defaults read "$1/Contents/Info.plist" CFBundleVersion 2>/dev/null || echo "$2")" = "$2" ]; do
              sleep 2
            done
            port=$3
            shift 3
            for label in "$@"; do /bin/launchctl kickstart -k "gui/$(id -u)/$label" && echo "$(date) restarted $label"; done
            for _ in $(seq 120); do
              /usr/bin/curl -fsS -o /dev/null "http://127.0.0.1:$port/pair" && { echo "$(date) cooee answers on $port"; exit 0; }
              sleep 0.5
            done
            echo "$(date) cooee does not answer on $port"
            exit 1
            """#

        internal static func restartAfterUpdate(port: Int) {
            let process = Process()
            process.executableURL = URL(filePath: "/bin/sh")
            let bundle = Bundle.main
            let version = bundle.object(forInfoDictionaryKey: "CFBundleVersion") as? String ?? ""
            process.arguments =
                ["-c", restartScript, "restart", bundle.bundleURL.path(percentEncoded: false), version, "\(port)"]
                + allCases.map(\.rawValue)
            let log = URL.homeDirectory.appending(path: "Library/Logs/cooee-update.log")
            if !FileManager.default.fileExists(atPath: log.path()) {
                FileManager.default.createFile(atPath: log.path(), contents: nil)
            }
            let handle = try? FileHandle(forWritingTo: log)
            _ = try? handle?.seekToEnd()
            process.standardOutput = handle
            process.standardError = handle
            try? process.run()
        }

        internal func register() -> ServiceState {
            let service = SMAppService.agent(plistName: "\(rawValue).plist")
            if service.status != .enabled { try? service.register() }
            switch service.status {
            case .enabled: return .enabled
            case .requiresApproval: return .needsApproval
            case .notFound: return .missing
            default: return .off
            }
        }

        internal func unregister() async throws {
            let service = SMAppService.agent(plistName: "\(rawValue).plist")
            if service.status != .notRegistered { try await service.unregister() }
        }
    }

    internal struct MacLive: Sendable {
        private let app = Bundle.main.bundleURL.appending(path: "Contents/Helpers")
        private let config = LocalConfig.standard

        private var server: LocalServer { LocalServer(cooee: app.appending(path: "cooee")) }

        internal func start() throws -> LoginItems {
            if config.value("shed") == nil {
                try config.set("shed", app.appending(path: "shed").path(percentEncoded: false))
            }
            if config.value("roots") == nil { try config.set("roots", ["~/Projects"]) }
            return LoginItems(cooee: LoginItem.cooee.register(), shed: LoginItem.shed.register())
        }

        internal func addFolder(_ path: String) async throws {
            let roots = config.strings("roots")
            guard !roots.contains(path) else { return }
            try config.set("roots", roots + [path])
            await restart()
        }

        internal func signIn(_ provider: String) async throws {
            let script = FileManager.default.temporaryDirectory.appending(path: "cooee-\(provider).command")
            try "#!/bin/zsh -l\nexec \(provider)\n".write(to: script, atomically: true, encoding: .utf8)
            try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: script.path())
            _ = try await Shell.run(
                URL(filePath: "/usr/bin/open"), ["-a", "Terminal", script.path(percentEncoded: false)])
        }

        internal func network() async throws -> Network {
            let network = await Shell.network()
            guard let name = network.tailnet else { return network }
            let hosts = config.strings("allowed_hosts")
            let port = config.port
            _ = try? await Shell.login("tailscale serve --bg --https=\(port) http://127.0.0.1:\(port)")
            guard !hosts.contains(name) else { return network }
            try config.set("allowed_hosts", hosts + [name])
            await restart()
            return network
        }

        internal func existing() async -> ExistingInstall {
            let launchctl = URL(filePath: "/bin/launchctl")
            let listed = (try? await Shell.run(launchctl, ["list"])) ?? ""
            let port = config.port
            let listener = try? await Shell.run(
                URL(filePath: "/usr/sbin/lsof"), ["-nP", "-iTCP:\(port)", "-sTCP:LISTEN", "-Fc"])
            return ExistingInstall(launchctl: listed, listener: listener ?? "", port: port)
        }

        internal func takeOver(_ labels: [String]) async throws {
            for label in labels {
                let target = "gui/\(getuid())/\(label)"
                _ = try await Shell.run(URL(filePath: "/bin/launchctl"), ["disable", target])
                _ = try await Shell.run(URL(filePath: "/bin/launchctl"), ["bootout", target])
            }
        }

        private func restart() async {
            let label = "gui/\(getuid())/\(LoginItem.cooee.rawValue)"
            _ = try? await Shell.run(URL(filePath: "/bin/launchctl"), ["kickstart", "-k", label])
        }

        internal func answer(_ id: String, yes: Bool) async throws {
            let context = LAContext()
            if yes && context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: nil) {
                try await context.evaluatePolicy(
                    .deviceOwnerAuthenticationWithBiometrics, localizedReason: "approve a new device for cooee")
            }
            try await server.answer(id, yes: yes)
        }

        internal func client() -> MacServerClient {
            MacServerClient(
                existing: { await existing() },
                useExisting: {
                    try await LoginItem.cooee.unregister()
                    try await LoginItem.shed.unregister()
                },
                takeOver: { try await takeOver($0) },
                start: { try start() },
                folders: { config.strings("roots") },
                addFolder: { try await addFolder($0) },
                agents: { await Shell.agents() },
                signIns: { try await server.signIns() },
                signIn: { try await signIn($0) },
                network: { try await network() },
                code: { try await server.code() },
                requests: { try await server.requests() },
                answer: { try await answer($0, yes: $1) }
            )
        }
    }

    extension MacServerClient: DependencyKey {
        public static let liveValue = MacLive().client()
    }
#endif
