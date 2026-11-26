import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension ConnectorsClient: DependencyKey {
    public static let liveValue = ConnectorsClient(
        list: { server in try await Client(server).connectors() },
        add: { server, name in try await Client(server).addConnector(name) },
        remove: { server, name in try await Client(server).removeConnector(name) },
        signIn: { server, name, account in try await Client(server).signIn(name, account: account) },
        flow: { server, id in try await Client(server).signInFlow(id) },
        paste: { server, name, flow, token in try await Client(server).paste(name, flow: flow, token: token) },
        signOut: { server, name, account in try await Client(server).signOut(name, account: account) }
    )
}
