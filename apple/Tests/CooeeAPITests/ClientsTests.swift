@testable import CooeeAPI
import CooeeCore
import Foundation
import Synchronization
import Testing

internal struct ClientsTests {
    private final class Factory: NSObject, URLSessionDelegate, Sendable {
        fileprivate let first: CannedServer
        fileprivate let second: CannedServer
        fileprivate let made = Mutex(0)
        fileprivate let invalidated: AsyncStream<Void>
        private let invalid: AsyncStream<Void>.Continuation

        fileprivate init(_ answer: Answer) {
            first = CannedServer(["/personas": [answer]])
            second = CannedServer(["/personas": [answer]])
            (invalidated, invalid) = AsyncStream.makeStream()
        }

        fileprivate func urlSession(_ session: URLSession, didBecomeInvalidWithError error: (any Error)?) {
            invalid.yield()
        }

        fileprivate func clients() -> Clients {
            Clients { server in
                self.made.withLock { $0 += 1 }
                let configuration = URLSessionConfiguration.ephemeral
                configuration.protocolClasses = [Canned.self]
                let session = URLSession(configuration: configuration, delegate: self, delegateQueue: nil)
                let base = (server.token == "first" ? self.first : self.second).client().base
                return Client(base: base, token: server.token, session: session)
            }
        }
    }

    private func server(_ token: String) -> Server {
        Server(url: URL(filePath: "/"), token: token)
    }

    @Test internal func fiftyCallsToOneServerMakeOneSession() async throws {
        let factory = Factory(try Fixture.answer("personas.json"))
        let clients = factory.clients()
        for _ in 0..<50 { _ = try await clients(server("first")).personas() }
        #expect(factory.made.withLock { $0 } == 1)
        #expect(factory.first.requests("/personas").count == 50)
    }

    @Test(.timeLimit(.minutes(1))) internal func switchingServerInvalidatesTheOldSession() async throws {
        let factory = Factory(try Fixture.answer("personas.json"))
        let clients = factory.clients()
        _ = try await clients(server("first")).personas()
        _ = try await clients(server("second")).personas()
        var invalidated = factory.invalidated.makeAsyncIterator()
        #expect(await invalidated.next() != nil)
        _ = try await clients(server("second")).personas()
        #expect(factory.made.withLock { $0 } == 2)
        #expect(factory.second.requests("/personas").count == 2)
    }
}
