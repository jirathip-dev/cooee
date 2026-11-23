import ComposableArchitecture
import CooeeCore

extension AppFeature {
    internal func surfaces() -> Effect<Action> {
        let (surface, serverStore) = (surface, serverStore)
        return .merge(
            .run { send in
                for await route in await surface.routes() {
                    switch route {
                    case .thread(let thread): await send(.open(thread))
                    case .start(let persona): await send(.start(persona))
                    }
                }
            },
            .run { _ in
                for await token in await surface.tokens() {
                    guard let server = await serverStore.load() else { continue }
                    try? await surface.register(server, token.token, token.activity)
                }
            }
        )
    }

    internal func keepGlance(_ state: State) -> Effect<Action> {
        guard let home = state.home else { return .none }
        let (surface, server, personas, style) = (surface, home.server, home.personas, state.menu.settings.style.kept)
        let threads = home.rows.filter { !$0.archived && $0.workflow == nil }.prefix(50).map { row in
            GlanceRow(
                thread: row.id, title: row.title, persona: row.persona,
                name: personas.first { $0.id == row.persona }?.name ?? row.agent, machine: row.machine ?? "",
                line: row.last, since: Int(row.updated ?? 0))
        }
        let host = server.url.host()?.split(separator: ".").first.map(String.init) ?? ""
        return .run { _ in
            guard let glance = try? await surface.glance(server) else { return }
            surface.keep(GlanceCache(glance: glance, personas: personas, threads: threads, host: host, style: style))
        }
    }
}
