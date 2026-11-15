import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension PersonasClient: DependencyKey {
    public static let liveValue = PersonasClient(
        create: { server, persona in try await Client(server).create(persona) },
        save: { server, persona in try await Client(server).save(persona) },
        memory: { server, persona in try await Client(server).memory(persona) },
        forget: { server, persona in try await Client(server).forget(persona) },
        agents: { server in try await Client(server).agents() },
        models: { server in try await Client(server).models() },
        folders: { server, path in try await Client(server).folders(path) }
    )
}
