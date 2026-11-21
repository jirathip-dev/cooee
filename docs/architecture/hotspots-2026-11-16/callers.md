# How the code reads afterwards

Written first: each block is what a lane does to add one thing, and the files it touches. Sketches, not final code.
One rule: **a new thing is a new file; `just gen` writes every list; nobody adds a line to a list.**

## A hub area that keeps state

Today: `mod inbox;`, a field on `Hub` and its line in `Hub::open`, all three in `hub.rs` (17 of the last 21 PRs on
`hub.rs` added one of them). Afterwards, one file:

```rust
// crates/cooee-server/src/hub/inbox.rs (new)
//! What waits for the owner.

use std::sync::Mutex;

use super::{Hub, HubError, Seed};

pub(super) struct State {
    waiting: Mutex<Vec<String>>,
}

impl State {
    pub(super) fn open(seed: &Seed<'_>) -> Result<Self, HubError> {
        Ok(Self { waiting: Mutex::new(load(&seed.data.join("inbox.json"))?) })
    }
}

impl Hub {
    /// The threads that wait for the owner.
    pub fn inbox(&self) -> Vec<String> {
        lock(&self.areas.inbox.waiting).clone()
    }
}
```

`just gen` adds `mod inbox;` to `hub.rs` and the field to `hub/areas.rs`. An area that needs nothing from the seed
derives `Default` and writes no `open`. `serve.rs` does not change: an area reads its settings from `seed.config`.

## A chat feature in the app, end to end

Today a pin on a message edits `Endpoints.swift`, `ThreadsClient.swift`, `ThreadsClient+Live.swift`,
`ChatFeature.swift`, `ChatActions.swift` and `ChatScreen.swift`. Afterwards it adds five files and runs `just gen`:

```text
apple/Sources/CooeeAPI/Endpoints/Pins.swift             extension Client { func pin(…) }
apple/Sources/CooeeFeatures/Clients/PinsClient.swift    its @DependencyClient
apple/Sources/CooeeApp/Live/PinsClient+Live.swift       its live value
apple/Sources/CooeeFeatures/Chat/ChatPin.swift          its state, actions and reducer function
apple/Sources/CooeeUI/ChatScreen/ChatPin.swift          its sheet, task and environment value
```

```swift
// apple/Sources/CooeeFeatures/Chat/ChatPin.swift
extension ChatFeature {
    @ObservableState
    public struct PinState: Equatable {
        public var pinned: Set<Int> = []
    }

    @CasePathable
    public enum PinAction: Sendable {
        case toggle(Int)
        case saved(Set<Int>)
    }

    internal func pin(_ state: inout State, _ action: Action) -> Effect<Action> {
        @Dependency(\.pins) var pins
        switch action {
        case .area(.pin(.toggle(let item))):
            state.area.pin.pinned.formSymmetricDifference([item])
            return .run { [server = state.server, thread = state.current] send in
                await send(.area(.pin(.saved(try await pins.toggle(server, thread, item)))))
            }
        case .area(.pin(.saved(let pinned))):
            state.area.pin.pinned = pinned
            return .none
        case .loaded:
            state.area.pin.pinned.formIntersection(state.items.map(\.id))
            return .none
        default:
            return .none
        }
    }
}
```

The function sees every action, so an area can react to the chat's own (`.loaded` above). The screen side:

```swift
// apple/Sources/CooeeUI/ChatScreen/ChatPin.swift
internal struct ChatPinPresenting: ViewModifier {
    internal let store: StoreOf<ChatFeature>

    internal func body(content: Content) -> some View {
        content.environment(
            \.pinActs, PinActs(pinned: store.area.pin.pinned) { store.send(.area(.pin(.toggle($0)))) })
    }
}

internal struct PinButton: View {
    @Environment(\.pinActs) private var acts
    internal let item: Int

    internal var body: some View {
        Button(acts.pinned.contains(item) ? "Unpin" : "Pin") { acts.toggle(item) }
    }
}
```

`PinActs` and its environment key sit in the same file, as `pickNight` does in `Sky.swift`. A design frame draws the
button with the key's default. A test reads `$0.area.pin.pinned` and sends `.area(.pin(.toggle(3)))`.

Placing `PinButton` in the header is still one line in `Chat/ChatHeader.swift`: a view's layout is code, not a list.

## An app feature with a screen of its own

The same two files under `CooeeFeatures/App/` and `CooeeUI/AppRoot/`. A child feature is scoped by the area:

```swift
// apple/Sources/CooeeFeatures/App/AppExport.swift
extension AppFeature {
    @ObservableState
    public struct ExportState: Equatable {
        @Presents public var sheet: ExportFeature.State?
    }

    @CasePathable
    public enum ExportAction: Sendable {
        case show
        case sheet(PresentationAction<ExportFeature.Action>)
    }

    internal var exportScope: some ReducerOf<Self> {
        Reduce { state, action in
            if case .area(.export(.show)) = action { state.area.export.sheet = ExportFeature.State() }
            return .none
        }
        .ifLet(\.area.export.$sheet, action: \.area.export.sheet) { ExportFeature() }
    }
}

// apple/Sources/CooeeUI/AppRoot/AppExport.swift
internal struct AppExportPresenting: ViewModifier {
    @Bindable internal var store: StoreOf<AppFeature>

    internal func body(content: Content) -> some View {
        content.sheet(item: $store.scope(state: \.area.export.sheet, action: \.area.export.sheet)) {
            ExportScreen(store: $0)
        }
    }
}
```

## A call on an old client

`ThreadsClient` is frozen: a line of it may change or go, none is added. A new call goes to its area's client
(`PinsClient` above), which the feature takes inside its function: `@Dependency(\.pins) var pins`.

## An architecture rule, a frozen path, a generated file

```text
.drover/arch/inbox.toml     [[ban]] … (tables only)
.drover/arch/inbox.list     generated crates/cooee-server/src/inbox.rs
scripts/gen.d/inbox         module crates/cooee-server/src/inbox
$ just gen                  writes .drover/arch.toml
```

Two PRs that each add a rule add two files. A conflict in `.drover/arch.toml` is fixed by `just gen`.

## A cap raise

```toml
# ci/cap.d/2026-11-24-801.toml (new; ci/cap.toml is frozen)
line_cap = 38600
why = "#801: owner, 2026-11-24: hub.rs's lines are counted since #729's hub slice"
```

The highest value of each key counts. `docs/cap-history/` keeps its page per raise, as today.

## A test, a doc section

As approved in #427: `crates/cooee/tests/mcp/<scenario>.rs` and `just gen`; here also
`docs/plugins/<section>.md`, which nothing lists.

## When a lane adds a line to a frozen list anyway

`drover arch-check` (in `drover turn done` and in CI) refuses it, run on a copy of this tree:

```text
apple/Sources/CooeeFeatures/ChatFeature.swift:58: `public var pinned: Set<Int> = []` added to a frozen list:
add one file per entry in `crates/cooee-core/src/*/*.rs` or … or `apple/Sources/CooeeFeatures/Chat/*.swift` …
```

It names every collected folder, not the one that fits. The folder is the list's own name: `ChatFeature.swift` and
`Chat/`, `AppFeature.swift` and `App/`, `ChatScreen.swift` and `ChatScreen/`, `Root.swift` and `AppRoot/`.
