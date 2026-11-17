import CooeeApp
import SwiftUI

@main
internal struct CooeeMain: App {
    #if os(iOS)
        @UIApplicationDelegateAdaptor(PushDelegate.self) private var delegate
    #else
        @NSApplicationDelegateAdaptor(PushDelegate.self) private var delegate
    #endif

    internal var body: some Scene {
        WindowGroup { CooeeRoot() }
    }
}
