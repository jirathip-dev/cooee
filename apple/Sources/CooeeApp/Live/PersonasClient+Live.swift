import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension PersonasClient: DependencyKey {
    public static let liveValue = PersonasClient(
        create: { server, persona in try await client(server).create(persona) },
        remove: { server, persona in try await client(server).remove(persona) },
        agents: { server in try await client(server).agents() },
        models: { server in try await client(server).models() },
        machines: { server in try await client(server).machines() },
        folders: { server, path in try await client(server).folders(path) }
    )
}
