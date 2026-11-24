import ComposableArchitecture
import CooeeFeatures
import SwiftUI

internal struct AppRootAreas: ViewModifier {
    internal let store: StoreOf<AppFeature>

    internal func body(content: Content) -> some View {
        content
    }
}
