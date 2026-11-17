import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension PluginsClient: DependencyKey {
    public static let liveValue = PluginsClient(
        places: { server, persona in try await Client(server).places(persona) },
        installed: { server in try await Client(server).plugins() },
        screen: { server, source, path in try await Client(server).screen(source, path) },
        events: { server, source, path in Client(server).screenEvents(source, path) },
        act: { server, source, screen, action in try await Client(server).act(source, screen: screen, action) },
        attach: { server, plugin, persona in try await Client(server).attach(plugin, to: persona) }
    )
}
