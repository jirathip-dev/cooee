import AuthenticationServices
import ComposableArchitecture
import CooeeFeatures
import Foundation

#if os(iOS)
    import UIKit
#else
    import AppKit
#endif

extension WebSignInClient: DependencyKey {
    public static let liveValue = WebSignInClient(open: { url, scheme in try await WebSheet.open(url, scheme) })
}

@MainActor
private final class WebSheet: NSObject, ASWebAuthenticationPresentationContextProviding {
    fileprivate static func open(_ url: URL, _ scheme: String) async throws {
        let sheet = WebSheet()
        try await withCheckedThrowingContinuation { (done: CheckedContinuation<Void, any Error>) in
            let session = ASWebAuthenticationSession(url: url, callback: .customScheme(scheme)) { _, error in
                withExtendedLifetime(sheet) { error.map { done.resume(throwing: $0) } ?? done.resume() }
            }
            session.presentationContextProvider = sheet
            session.start()
        }
    }

    fileprivate func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
        #if os(iOS)
            let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
            return scenes.flatMap(\.windows).first(where: \.isKeyWindow) ?? ASPresentationAnchor()
        #else
            return NSApplication.shared.keyWindow ?? ASPresentationAnchor()
        #endif
    }
}
