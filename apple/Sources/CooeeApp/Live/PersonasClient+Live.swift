import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension PersonasClient: DependencyKey {
    public static let liveValue = PersonasClient(
        create: { server, persona in try await Client(server).create(persona) },
        remove: { server, persona in try await Client(server).remove(persona) },
        agents: { server in try await Client(server).agents() },
        models: { server in try await Client(server).models() },
        machines: { server in try await Client(server).machines() },
        folders: { server, path in try await Client(server).folders(path) }
    )
}
