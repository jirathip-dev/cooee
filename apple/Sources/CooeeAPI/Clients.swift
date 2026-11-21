import CooeeCore
import Synchronization

/// One `Client`, and so one URL session, per server, replaced when the server or its token changes.
public final class Clients: Sendable {
    private let make: @Sendable (Server) -> Client
    private let held = Mutex<(server: Server, client: Client)?>(nil)

    public init(make: @escaping @Sendable (Server) -> Client = { Client($0) }) {
        self.make = make
    }

    public func callAsFunction(_ server: Server) -> Client {
        held.withLock { held in
            if let held, held.server == server { return held.client }
            let client = make(server)
            held = (server, client)
            return client
        }
    }
}
