import CooeeUI
import SwiftUI

/// The app's root: onboarding until paired, then Home. iPhone pushes a chat over Home; Mac and iPad show the chat
/// list as a sidebar beside the open chat. Fixtures only (#124); the API client comes with the build issues.
public struct RootView: View {
    @Environment(\.colorScheme) var scheme
    #if os(iOS)
    @Environment(\.horizontalSizeClass) var size
    #else
    let size: UserInterfaceSizeClass? = .regular
    #endif
    @State var paired = true
    @State var style: LookStyle = .modern
    @State var menu = false
    @State var persona: String? = nil
    @State var chat: String? = "t1"

    public init() {}

    public var body: some View {
        Group {
            if !paired {
                PhoneOnboarding(step: .welcome)
            } else if size == .compact {
                // iPhone: Home, a chat pushed over it.
                NavigationStack {
                    HomeView()
                        .navigationDestination(for: String.self) { id in chatView(id).toolbar(.hidden) }
                        .toolbar(.hidden)
                }
            } else {
                NavigationSplitView {
                    HomeView()
                } detail: {
                    chatView(chat ?? "t1")
                }
            }
        }
        .sheet(isPresented: $menu) { AvatarMenu().presentationDetents([.medium, .large]) }
        .sheet(item: Binding(get: { persona.map(Ident.init) }, set: { persona = $0?.id })) { PersonaSheet(personaID: $0.id) }
        .environment(\.look, Look(palette: .farm(scheme), style: style))
    }

    func chatView(_ id: String) -> ChatView {
        id == "t2" ? .cards() : .streaming()
    }
}

struct Ident: Identifiable { let id: String }

#Preview("Home") { HomeView().environment(\.look, Look(palette: .farmDark)) }
#Preview("Chat") { ChatView.streaming().environment(\.look, Look(palette: .farmDark)) }
#Preview("Cards") { ChatView.cards().environment(\.look, Look(palette: .farmLight)) }
#Preview("Persona") { PersonaSheet().environment(\.look, Look(palette: .farmDark)) }
#Preview("Menu") { HomeView(menuOpen: true).environment(\.look, Look(palette: .farmDark)) }
#Preview("Terminal") { HomeView().environment(\.look, Look(palette: .farmDark, style: .terminal)) }
#Preview("Scene") { HomeView().environment(\.look, Look(palette: .farmLight, style: .scene)) }
#Preview("Pair") { PhoneOnboarding(step: .welcome).environment(\.look, Look(palette: .farmDark)) }
#Preview("Mac setup") { MacSetup(step: .qr).frame(width: 960, height: 640).environment(\.look, Look(palette: .farmDark)) }
