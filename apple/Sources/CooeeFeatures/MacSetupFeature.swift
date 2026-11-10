import ComposableArchitecture
import CooeeCore
import Foundation

public enum MacStep: String, CaseIterable, Sendable {
    case welcome, folders, signins, tailscale
    case pairPhone = "qr"
    case approve
}

@Reducer
public struct MacSetupFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public var step: MacStep
        public var items: LoginItems?
        public var folders: [String] = []
        public var agents: [FoundAgent] = []
        public var signIns: [SignIn] = []
        public var network: Network?
        public var code: PairCode?
        public var request: PairRequest?
        public var answered: [String] = []
        public var approved = 0
        public var error: String?

        public init(step: MacStep = .welcome) {
            self.step = step
        }
    }

    public enum Action: Sendable {
        case setUp
        case started(LoginItems)
        case foldersLoaded([String])
        case agentsLoaded([FoundAgent])
        case signInsLoaded([SignIn])
        case networkChecked(Network)
        case codeShown(PairCode)
        case failed(String)
        case addFolder(String)
        case signIn(String)
        case checkNetwork
        case next
        case back
        case requested([PairRequest])
        case answer(Bool)
        case done
        case connectElsewhere
        case delegate(Delegate)

        @CasePathable
        public enum Delegate: Sendable, Equatable {
            case paired(Server)
            case clientOnly
        }
    }

    private enum CancelID {
        case polling
    }

    private static let order: [MacStep] = [.welcome, .folders, .signins, .tailscale, .pairPhone]

    @Dependency(\.mac) private var mac
    @Dependency(\.pairing) private var pairing
    @Dependency(\.serverStore) private var serverStore
    @Dependency(\.continuousClock) private var clock

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .setUp:
                return setUp(&state)
            case .next:
                return move(&state, by: 1)
            case .back:
                return move(&state, by: -1)
            case .requested(let requests):
                return requested(&state, requests)
            case .answer(let yes):
                return answer(&state, yes)
            case .done:
                return done()
            case .connectElsewhere:
                return .send(.delegate(.clientOnly))
            default:
                return loaded(&state, action)
            }
        }
    }
}

extension MacSetupFeature {
    private static func report(_ error: any Error, _ send: Send<Action>) async {
        await send(.failed(error.localizedDescription))
    }

    private func loaded(_ state: inout State, _ action: Action) -> Effect<Action> {
        switch action {
        case .started(let items):
            state.items = items
            state.step = .folders
        case .foldersLoaded(let folders):
            state.folders = folders
        case .agentsLoaded(let agents):
            state.agents = agents
        case .signInsLoaded(let signIns):
            state.signIns = signIns
        case .networkChecked(let network):
            state.network = network
        case .codeShown(let code):
            state.code = code
        case .failed(let error):
            state.error = error
        default:
            return command(action)
        }
        return .none
    }

    private func command(_ action: Action) -> Effect<Action> {
        let mac = mac
        switch action {
        case .addFolder(let path):
            return .run { send in
                try await mac.addFolder(path)
                await send(.foldersLoaded(await mac.folders()))
            } catch: {
                await Self.report($0, $1)
            }
        case .signIn(let provider):
            return .run { _ in
                try await mac.signIn(provider)
            } catch: {
                await Self.report($0, $1)
            }
        case .checkNetwork:
            return .run { send in
                await send(.networkChecked(try await mac.network()))
            } catch: {
                await Self.report($0, $1)
            }
        default:
            return .none
        }
    }

    private func setUp(_ state: inout State) -> Effect<Action> {
        state.error = nil
        let mac = mac
        return .run { send in
            await send(.started(try await mac.start()))
            await send(.foldersLoaded(await mac.folders()))
            await send(.agentsLoaded(await mac.agents()))
        } catch: {
            await Self.report($0, $1)
        }
    }

    private func move(_ state: inout State, by offset: Int) -> Effect<Action> {
        guard let index = Self.order.firstIndex(of: state.step) else { return .none }
        let step = Self.order[max(0, min(Self.order.count - 1, index + offset))]
        state.step = step
        state.error = nil
        let mac = mac
        switch step {
        case .pairPhone:
            return poll()
        case .signins:
            return .merge(
                .cancel(id: CancelID.polling),
                .run { send in
                    await send(.signInsLoaded(try await mac.signIns()))
                } catch: {
                    await Self.report($0, $1)
                }
            )
        case .tailscale:
            return .merge(.cancel(id: CancelID.polling), .send(.checkNetwork))
        default:
            return .cancel(id: CancelID.polling)
        }
    }

    private func poll() -> Effect<Action> {
        let (mac, clock) = (mac, clock)
        let polling = Effect<Action>.run(
            operation: { send in
                await send(.codeShown(try await mac.code()))
                while true {
                    await send(.requested(try await mac.requests()))
                    try await clock.sleep(for: .seconds(2))
                }
            },
            catch: { await Self.report($0, $1) }
        )
        return polling.cancellable(id: CancelID.polling, cancelInFlight: true)
    }

    private func requested(_ state: inout State, _ requests: [PairRequest]) -> Effect<Action> {
        let answered = state.answered
        let open = requests.filter { !answered.contains($0.id) }
        if state.step == .approve, let shown = state.request, !open.contains(shown) {
            state.request = nil
            state.step = .pairPhone
        }
        if state.step == .pairPhone, let first = open.first {
            state.request = first
            state.step = .approve
        }
        return .none
    }

    private func answer(_ state: inout State, _ yes: Bool) -> Effect<Action> {
        guard let request = state.request else { return .none }
        state.answered.append(request.id)
        state.request = nil
        state.step = .pairPhone
        state.approved += yes ? 1 : 0
        let mac = mac
        return .run { _ in
            try await mac.answer(request.id, yes)
        } catch: {
            await Self.report($0, $1)
        }
    }

    private func done() -> Effect<Action> {
        let (mac, pairing, serverStore) = (mac, pairing, serverStore)
        return .merge(
            .cancel(id: CancelID.polling),
            .run { send in
                let code = try await mac.code()
                let server = Server(url: code.local, token: try await pairing.pair(code.local, code.code))
                serverStore.save(server)
                await send(.delegate(.paired(server)))
            } catch: {
                await Self.report($0, $1)
            }
        )
    }
}
