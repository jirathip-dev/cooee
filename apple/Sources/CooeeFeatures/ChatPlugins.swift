import ComposableArchitecture
import CooeeCore

private enum Deck { case follow }

extension ChatFeature {
    @CasePathable
    public enum WorkflowAction: Sendable, Equatable {
        case show(Bool)
        case unfold(String)
        case open(String)
    }

    @CasePathable
    public enum PluginAction: Sendable, Equatable {
        case load
        case loaded([Place], [PluginRow])
        case teach([JSONValue])
        case counted(String, JSONValue)
        case open(Place)
        case attach(PluginRow)
    }

    internal func plugins(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .plugins(let action):
            return plugin(&state, action)
        case .screen(.presented(.delegate(.open(let thread)))):
            state.screen = nil
            return .send(.delegate(.open(thread)))
        case .attaching(.presented(.delegate(.attached))):
            state.attaching = nil
            return .send(.plugins(.load))
        case .workflow(.show(let open)):
            state.workflowOpen = open
        case .workflow(.unfold(let issue)):
            state.unfolded = state.unfolded == issue ? nil : issue
        case .workflow(.open(let thread)):
            state.workflowOpen = false
            return .send(.delegate(.round(thread)))
        default:
            break
        }
        return .none
    }

    private func plugin(_ state: inout State, _ action: PluginAction) -> Effect<Action> {
        switch action {
        case .load:
            return load(state)
        case .loaded(let places, let installed):
            state.places = places.filter { !$0.cards }
            state.deck.follow(places.filter(\.cards))
            state.attachable = installed.filter { row in
                row.places.contains { $0.kind == "header-strip" } && !places.contains { $0.plugin == row.name }
            }
        case .teach(let rows):
            state.teach = rows
        case .counted(let place, let event):
            state.strips[place, default: LiveScreen(.null)].apply(event: event)
        case .open(let place):
            state.screen = ScreenFeature.State(server: state.server, place: place)
        case .attach(let plugin):
            state.attaching = AttachFeature.State(server: state.server, persona: state.persona.api, plugin: plugin)
        }
        return .none
    }

    private func load(_ state: State) -> Effect<Action> {
        let (plugins, server, persona, clock) = (pluginAPI, state.server, state.persona.api.id, clock)
        guard !persona.isEmpty else { return .none }
        return .run { send in
            async let places = plugins.places(server, persona)
            async let installed = plugins.installed(server)
            let found = try await places
            await send(.plugins(.loaded(found, try await installed)))
            let screen = try await plugins.screen(server, .core, "/screens/persona/\(persona)")
            await send(.plugins(.teach(screen.list("blocks").first { $0.type == "actions" }?.list("rows") ?? [])))
            await Self.deal(found, plugins, clock, server, send)
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
        .cancellable(id: Deck.follow, cancelInFlight: true)
    }
}
