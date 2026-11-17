import ComposableArchitecture
import CooeeCore
import Foundation

@Reducer
public struct ThemeEditorFeature: Sendable {
    @ObservableState
    public struct State: Equatable {
        public var server: Server
        public var draft: ThemeDraft
        public var kept: String?
        public var art: URL?
        public var schemes: [String: ThemeScheme]
        public var words: [String: String]
        public var layers: Int
        public var reports: [String: GateReport] = [:]
        public var colours: Bool?
        public var open: String?
        public var drawing = Drawing()
        public var chat = false
        public var dark = true
        public var saving = false
        public var error: String?

        public init(server: Server, looks: Looks, theme: String, style: LookStyle, new: Bool) {
            let own = !new && !ThemeDraft.builtIn.contains(theme) && looks.themes[theme] != nil
            draft = ThemeDraft(looks: looks, theme: theme, style: style, own: own, new: new)
            self.server = server
            kept = own ? theme : nil
            art = new ? nil : looks.art(theme, on: server.url)
            schemes = looks.schemes
            let pack = new ? nil : looks.themes[theme]
            words = pack?.words ?? [:]
            layers = pack?.scene?.count ?? 0
        }

        public var name: String { kept ?? ThemeDraft.folder(draft.label) }

        public var fails: [String] { ["dark", "light"].flatMap { reports[$0]?.fails ?? [] } }

        public var canSave: Bool {
            reports.count == 2 && fails.isEmpty && !name.isEmpty && !ThemeDraft.builtIn.contains(name) && !saving
        }

        public func tokens(dark: Bool) -> [String: String] { dark ? draft.dark : draft.light }
    }

    public enum Action: Sendable {
        case task
        case label(String)
        case style(LookStyle)
        case colours(Bool?)
        case open(String?)
        case preview(chat: Bool, dark: Bool)
        case token(String, String)
        case draw(DrawEdit)
        case art(sprites: String, scene: String)
        case start(String)
        case checked([String: GateReport])
        case save
        case failed(String)
        case saved(String, LookStyle)
    }

    @Dependency(\.threads) private var threads

    public init() {}

    public var body: some ReducerOf<Self> {
        Reduce { state, action in
            switch action {
            case .task:
                return .concatenate(load(state), check(state))
            case .draw(let edit):
                state.drawing.apply(edit)
            case .art(let sprites, let scene):
                state.drawing.figures = PixelFigure.drawn(sprites: sprites, scene: scene)
            case .token, .start:
                return edit(&state, action)
            case .checked(let reports):
                state.reports = reports
            case .save:
                return save(&state)
            case .failed(let error):
                state.saving = false
                state.error = error
            default:
                show(&state, action)
            }
            return .none
        }
    }

    private func show(_ state: inout State, _ action: Action) {
        switch action {
        case .label(let label): state.draft.label = label
        case .style(let style): state.draft.style = style
        case .colours(let dark):
            state.colours = dark
            state.open = nil
        case .open(let token): state.open = token
        case .preview(let chat, let dark): (state.chat, state.dark) = (chat, dark)
        default: break
        }
    }

    private func edit(_ state: inout State, _ action: Action) -> Effect<Action> {
        guard let dark = state.colours else { return .none }
        var tokens = state.tokens(dark: dark)
        switch action {
        case .token(let key, let value): tokens[key] = value
        case .start(let scheme): tokens = state.schemes[scheme]?.tokens ?? tokens
        default: break
        }
        if dark { state.draft.dark = tokens } else { state.draft.light = tokens }
        return check(state)
    }

    private func check(_ state: State) -> Effect<Action> {
        let (server, draft) = (state.server, state.draft)
        return .run { send in
            await send(.checked(try await threads.checkTheme(server, draft)))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
        .cancellable(id: "check", cancelInFlight: true)
    }

    private func load(_ state: State) -> Effect<Action> {
        let (server, from) = (state.server, state.draft.from)
        guard from != "starter" else { return .none }
        return .run { send in
            async let sprites = threads.themeArt(server, from, "sprites.svg")
            async let scene = threads.themeArt(server, from, "scene.svg")
            await send(.art(sprites: (try? await sprites) ?? "", scene: (try? await scene) ?? ""))
        }
    }

    private func save(_ state: inout State) -> Effect<Action> {
        guard state.canSave else { return .none }
        state.draft.draw(state.drawing.figures)
        state.saving = true
        state.error = nil
        let (server, name, draft) = (state.server, state.name, state.draft)
        return .run { send in
            try await threads.saveTheme(server, name, draft)
            await send(.saved(name, draft.style))
        } catch: { error, send in
            await send(.failed(error.localizedDescription))
        }
    }
}
