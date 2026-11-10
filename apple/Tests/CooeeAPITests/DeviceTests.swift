@testable import CooeeAPI
import CooeeCore
import Foundation
import Network
import Testing

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

    @Test(.timeLimit(.minutes(1)))
    internal func aServerAdvertisedOnTheLANIsFoundAtItsAddress() async throws {
        let type = "_cooee-\(UUID().uuidString.prefix(6).lowercased())._tcp"
        let listener = try NWListener(using: .tcp)
        listener.service = NWListener.Service(name: "mini-test", type: type)
        listener.newConnectionHandler = { $0.cancel() }
        listener.start(queue: .global())
        defer { listener.cancel() }
        var found: [NearbyServer] = []
        for await servers in Nearby.browse(type: type) where servers.contains(where: { $0.name == "mini-test" }) {
            found = servers
            break
        }
        let server = try #require(found.first { $0.name == "mini-test" })
        #expect(server.address.hasPrefix("http://") && server.address.hasSuffix(":\(listener.port?.rawValue ?? 0)"))
    }
}
