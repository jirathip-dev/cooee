import ComposableArchitecture
import CooeeCore
import Foundation

public enum PhoneStep: String, CaseIterable, Sendable {
    case welcome, scan, nearby, address, waiting, paired
}

@Reducer
public struct PairingFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public var step: PhoneStep = .welcome
        public var address = ""
        public var word = ""
        public var until = Date.distantPast
        public var error: String?
        public var host = ""
        public var url: URL?
        public var claim: String?
        public var server: Server?
        public var personas: [APIPersona] = []
        public var nearby: [NearbyServer] = []
        public var browsing = false

        public init() {}
    }

    public enum Action: Sendable {
        case go(PhoneStep)
        case addressChanged(String)
        case connect(String)
        case asked(PairAsk)
        case claimed(PairClaim)
        case paired(Server)
        case personasLoaded([APIPersona])
        case failed(String)
        case cancel
        case home
        case browse
        case found([NearbyServer])
        case delegate(Delegate)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case paired(Server)
        }
    }

    private enum CancelID {
        case pairing
    }

    @Dependency(\.pairing) private var pairing
    @Dependency(\.threads) private var threads
    @Dependency(\.serverStore) private var serverStore
    @Dependency(\.nearby) private var nearby
    @Dependency(\.continuousClock) private var clock
    @Dependency(\.date.now) private var now

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .go(let step):
                state.step = step
                state.error = nil
                return .none
            case .addressChanged(let address):
                state.address = address
                return .none
            case .connect(let typed):
                return connect(&state, typed)
            case .asked(let ask):
                return asked(&state, ask)
            case .claimed(let claim):
                return claimed(&state, claim)
            case .paired(let server):
                return paired(&state, server)
            default:
                return other(&state, action)
            }
        }
    }

    private func other(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .personasLoaded(let personas):
            state.personas = personas
        case .failed(let error):
            state.error = error
        case .cancel:
            return cancel(&state)
        case .home:
            return state.server.map { .send(.delegate(.paired($0))) } ?? .none
        case .browse:
            return browse(&state)
        case .found(let servers):
            state.nearby = servers
        default:
            break
        }
        return .none
    }

    private func connect(_ state: inout State, _ typed: String) -> Effect<Action> {
        state.error = nil
        guard let target = PairTarget(typed: typed) else {
            state.error = "Type an address, like server.example.com."
            return .none
        }
        state.url = target.url
        state.host = target.url.host() ?? ""
        let pairing = pairing
        return .run { send in
            if let code = target.code {
                await send(.paired(Server(url: target.url, token: try await pairing.pair(target.url, code))))
            } else {
                await send(.asked(try await pairing.ask(target.url)))
            }
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
        .cancellable(id: CancelID.pairing, cancelInFlight: true)
    }

    private func asked(_ state: inout State, _ ask: PairAsk) -> Effect<Action> {
        guard let url = state.url else { return .none }
        state.word = ask.word
        state.until = now.addingTimeInterval(300)
        state.claim = ask.claim
        state.step = .waiting
        let (pairing, clock) = (pairing, clock)
        return .run { send in
            while true {
                let answer = try await pairing.claim(url, ask.claim)
                guard answer == .waiting else { return await send(.claimed(answer)) }
                try await clock.sleep(for: .seconds(2))
            }
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
        .cancellable(id: CancelID.pairing, cancelInFlight: true)
    }

    private func claimed(_ state: inout State, _ claim: PairClaim) -> Effect<Action> {
        switch claim {
        case .paired(let token):
            guard let url = state.url else { return .none }
            return paired(&state, Server(url: url, token: token))
        case .declined(let why), .gone(let why):
            state.error = why
            state.claim = nil
            state.step = .address
            return .none
        case .waiting:
            return .none
        }
    }

    private func paired(_ state: inout State, _ server: Server) -> Effect<Action> {
        serverStore.save(server)
        state.server = server
        state.claim = nil
        state.step = .paired
        let threads = threads
        return .run { send in
            await send(.personasLoaded((try? await threads.personas(server)) ?? []))
        }
    }

    private func cancel(_ state: inout State) -> Effect<Action> {
        state.step = .welcome
        guard let claim = state.claim, let url = state.url else { return .cancel(id: CancelID.pairing) }
        state.claim = nil
        let pairing = pairing
        return .merge(
            .cancel(id: CancelID.pairing),
            .run { _ in try? await pairing.cancel(url, claim) }
        )
    }

    private func browse(_ state: inout State) -> Effect<Action> {
        guard !state.browsing else { return .none }
        state.browsing = true
        let nearby = nearby
        return .run { send in
            for await servers in nearby.browse() { await send(.found(servers)) }
        }
    }
}
