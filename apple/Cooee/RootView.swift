import CooeeUI
import SwiftUI

/// The app's root (`AppRoot`): onboarding until paired, then Home and chats against the server (#143).
public struct RootView: View {
    public init() {}
    public var body: some View { AppRoot() }
}

#Preview("Home") { HomeView().environment(\.look, Look(palette: .farmDark)) }
#Preview("Chat") { ChatView.streaming().environment(\.look, Look(palette: .farmDark)) }
#Preview("Cards") { ChatView.cards().environment(\.look, Look(palette: .farmLight)) }
#Preview("Persona") { PersonaSheet().environment(\.look, Look(palette: .farmDark)) }
#Preview("Menu") { HomeView(menuOpen: true).environment(\.look, Look(palette: .farmDark)) }
#Preview("Terminal") { HomeView().environment(\.look, Look(palette: .farmDark, style: .terminal)) }
#Preview("Scene") { HomeView().environment(\.look, Look(palette: .farmLight, style: .scene)) }
#Preview("Pair") { PhoneOnboarding(step: .welcome).environment(\.look, Look(palette: .farmDark)) }
#Preview("Mac setup") { MacSetup(step: .qr).frame(width: 960, height: 640).environment(\.look, Look(palette: .farmDark)) }
