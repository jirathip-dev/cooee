#if os(macOS)
    import ComposableArchitecture
    import CooeeAPI
    import CooeeCore
    import CooeeFeatures
    import Foundation
    import LocalAuthentication
    import ServiceManagement

    internal enum LoginItem: String {
        case cooee = "com.jirathip.cooee.serve"
        case shed = "com.jirathip.cooee.shed"

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
    }

    internal struct MacLive: Sendable {
        private let app = Bundle.main.bundleURL.appending(path: "Contents/MacOS")
        private let config = LocalConfig.standard

        private var server: LocalServer { LocalServer(cooee: app.appending(path: "cooee")) }

        internal func start() throws -> LoginItems {
            if config.value("shed") == nil {
                try config.set("shed", app.appending(path: "shed").path(percentEncoded: false))
            }
            if config.value("roots") == nil { try config.set("roots", ["~/Projects"]) }
            return LoginItems(cooee: LoginItem.cooee.register(), shed: LoginItem.shed.register())
        }

        internal func addFolder(_ path: String) throws {
            let roots = config.strings("roots")
            guard !roots.contains(path) else { return }
            try config.set("roots", roots + [path])
            restart()
        }

        internal func signIn(_ provider: String) throws {
            let script = FileManager.default.temporaryDirectory.appending(path: "cooee-\(provider).command")
            try "#!/bin/zsh -l\nexec \(provider)\n".write(to: script, atomically: true, encoding: .utf8)
            try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: script.path())
            _ = try Shell.run(URL(filePath: "/usr/bin/open"), ["-a", "Terminal", script.path(percentEncoded: false)])
        }

        internal func network() throws -> Network {
            let network = Shell.network()
            guard let name = network.tailnet else { return network }
            let hosts = config.strings("allowed_hosts")
            let port = config.port
            _ = try? Shell.login("tailscale serve --bg --https=\(port) http://127.0.0.1:\(port)")
            guard !hosts.contains(name) else { return network }
            try config.set("allowed_hosts", hosts + [name])
            restart()
            return network
        }

        private func restart() {
            let label = "gui/\(getuid())/\(LoginItem.cooee.rawValue)"
            _ = try? Shell.run(URL(filePath: "/bin/launchctl"), ["kickstart", "-k", label])
        }

        internal func answer(_ id: String, yes: Bool) async throws {
            let context = LAContext()
            if yes && context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: nil) {
                try await context.evaluatePolicy(
                    .deviceOwnerAuthenticationWithBiometrics, localizedReason: "approve a new device for cooee")
            }
            try server.answer(id, yes: yes)
        }

        internal func client() -> MacServerClient {
            MacServerClient(
                start: { try start() },
                folders: { config.strings("roots") },
                addFolder: { try addFolder($0) },
                agents: { Shell.agents() },
                signIns: { try server.signIns() },
                signIn: { try signIn($0) },
                network: { try network() },
                code: { try server.code() },
                requests: { try server.requests() },
                answer: { try await answer($0, yes: $1) }
            )
        }
    }

    extension MacServerClient: DependencyKey {
        public static let liveValue = MacLive().client()
    }
#endif
