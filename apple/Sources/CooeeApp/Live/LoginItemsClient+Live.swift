import ComposableArchitecture
import CooeeFeatures

#if os(macOS)
    import CooeeAPI
    import Foundation
    import ServiceManagement

    extension LoginItemsClient: DependencyKey {
        public static let liveValue = LoginItemsClient {
            var retired = false
            for label in ["com.jirathip.cooee.serve", "com.jirathip.cooee.shed"] {
                let job = "gui/\(getuid())/\(label)"
                if (try? await Shell.run(URL(filePath: "/bin/launchctl"), ["bootout", job])) != nil { retired = true }
                try? await SMAppService.agent(plistName: "\(label).plist").unregister()
            }
            return retired
        }
    }
#else
    extension LoginItemsClient: DependencyKey {
        public static let liveValue = LoginItemsClient { false }
    }
#endif
