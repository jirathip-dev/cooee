import CooeeApp
import SwiftUI

@main
internal struct CooeeMain: App {
    #if os(iOS)
        @UIApplicationDelegateAdaptor(PushDelegate.self) private var delegate
    #else
        @NSApplicationDelegateAdaptor(PushDelegate.self) private var delegate
    #endif

    internal init() { CooeeShortcuts.updateAppShortcutParameters() }

    internal var body: some Scene {
        #if os(macOS)
            WindowGroup(id: CooeeMenuBar.window) { CooeeRoot() }
            CooeeMenuBar()
        #else
            WindowGroup { CooeeRoot() }
        #endif
    }
}
