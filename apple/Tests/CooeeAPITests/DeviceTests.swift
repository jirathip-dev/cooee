import CooeeCore
import Foundation
import Network
import Testing

@testable import CooeeAPI

internal struct DeviceTests {
    @Test internal func theKeychainKeepsOneServer() throws {
        let keychain = Keychain(service: "dev.cooee.tests.\(UUID().uuidString)")
        defer { keychain.clear() }
        #expect(keychain.load() == nil)
        let server = Server(url: try #require(URL(string: "https://mini.local")), token: "t1")
        try #require(keychain.save(server))
        #expect(keychain.load() == server)
        let next = Server(url: server.url, token: "t2")
        #expect(keychain.save(next))
        #expect(keychain.load() == next)
        keychain.clear()
        #expect(keychain.load() == nil)
    }

    @Test internal func aResolvedServerIsAnHTTPAddress() throws {
        let port = try #require(NWEndpoint.Port(rawValue: 4100))
        #expect(Nearby.address(.hostPort(host: "192.0.2.20", port: port)) == "http://192.0.2.20:4100")
        #expect(Nearby.address(.hostPort(host: "fe80::1%en0", port: port)) == "http://fe80::1:4100")
        #expect(Nearby.address(.service(name: "mini", type: "_cooee._tcp", domain: "local", interface: nil)) == nil)
    }

    @Test internal func browsingStartsAndStops() async {
        let stream = Nearby.browse(type: "_cooee-test-\(UUID().uuidString.prefix(8))._tcp")
        let reader = Task {
            for await _ in stream {}
        }
        try? await Task.sleep(for: .milliseconds(50))
        reader.cancel()
        await reader.value
    }
}
