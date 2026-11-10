import CooeeCore
import Foundation
import Network
import Synchronization

public final class Nearby: Sendable {
    private let browser: NWBrowser
    private let found = Mutex<[NearbyServer]>([])
    private let queue = DispatchQueue(label: "dev.cooee.nearby")

    public init(type: String = "_cooee._tcp") {
        browser = NWBrowser(for: .bonjour(type: type, domain: nil), using: .tcp)
    }

    public static func browse(type: String = "_cooee._tcp") -> AsyncStream<[NearbyServer]> {
        AsyncStream { continuation in
            let nearby = Nearby(type: type)
            nearby.start { continuation.yield($0) }
            continuation.onTermination = { _ in nearby.browser.cancel() }
        }
    }

    private func start(_ report: @escaping @Sendable ([NearbyServer]) -> Void) {
        browser.browseResultsChangedHandler = { [self] results, _ in
            for result in results { resolve(result.endpoint, report) }
        }
        browser.start(queue: queue)
    }

    private func resolve(_ endpoint: NWEndpoint, _ report: @escaping @Sendable ([NearbyServer]) -> Void) {
        guard case .service(let name, _, _, _) = endpoint, !known(name) else { return }
        let connection = NWConnection(to: endpoint, using: .tcp)
        connection.stateUpdateHandler = { [self] state in
            guard case .ready = state, let remote = connection.currentPath?.remoteEndpoint,
                let address = Self.address(remote)
            else { return }
            connection.cancel()
            let list = found.withLock { list in
                if !list.contains(where: { $0.name == name }) { list.append(NearbyServer(name: name, address: address)) }
                return list
            }
            report(list)
        }
        connection.start(queue: queue)
    }

    private func known(_ name: String) -> Bool {
        found.withLock { $0.contains { $0.name == name } }
    }

    internal static func address(_ endpoint: NWEndpoint) -> String? {
        guard case .hostPort(let host, let port) = endpoint else { return nil }
        let text = "\(host)"
        return "http://\(text.split(separator: "%").first.map(String.init) ?? text):\(port)"
    }
}
