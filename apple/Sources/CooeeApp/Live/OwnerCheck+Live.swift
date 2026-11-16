import ComposableArchitecture
import CooeeFeatures

#if os(iOS)
    import LocalAuthentication
#endif

extension OwnerCheck: DependencyKey {
    public static let liveValue = OwnerCheck { reason in
        #if os(iOS)
            let context = LAContext()
            guard context.canEvaluatePolicy(.deviceOwnerAuthentication, error: nil) else { return true }
            return (try? await context.evaluatePolicy(.deviceOwnerAuthentication, localizedReason: reason)) ?? false
        #else
            return true
        #endif
    }
}
