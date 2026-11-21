import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension PluginsClient: DependencyKey {
    public static let liveValue = PluginsClient(
        places: { server, persona in try await client(server).places(persona) },
        installed: { server in try await client(server).plugins() },
        screen: { server, source, path in try await client(server).screen(source, path) },
        events: { server, source, path in client(server).screenEvents(source, path) },
        act: { server, source, screen, action in try await client(server).act(source, screen: screen, action) },
        attach: { server, plugin, persona in try await client(server).attach(plugin, to: persona) }
    )
}
