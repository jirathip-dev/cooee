import ComposableArchitecture
import CooeeFeatures
import SwiftUI

public struct UpdateSettings: View {
    private let store: StoreOf<UpdatesFeature>

    public init(store: StoreOf<UpdatesFeature>) {
        self.store = store
    }

    public var body: some View {
        Form {
            switch store.status {
            case .unsigned:
                Text("updates off: unsigned build")
            case .signed(let automatic):
                Toggle(
                    "Automatically update", isOn: Binding(get: { automatic }, set: { store.send(.setAutomatic($0)) }))
            }
        }
        .padding(20)
        .frame(width: 360)
    }
}

public struct UpdateCommands: Commands {
    private let store: StoreOf<UpdatesFeature>

    public init(store: StoreOf<UpdatesFeature>) {
        self.store = store
    }

    public var body: some Commands {
        CommandGroup(after: .appInfo) {
            Button("Check for Updates…") { store.send(.check) }
                .disabled(store.status == .unsigned)
        }
    }
}
