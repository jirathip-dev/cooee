import CooeeUI
import SwiftUI

#Preview("Home") { HomeView().environment(\.look, Look(palette: .farmDark)) }
#Preview("Chat") { ChatView.streaming().environment(\.look, Look(palette: .farmDark)) }
#Preview("Cards") { ChatView.cards().environment(\.look, Look(palette: .farmLight)) }
#Preview("Persona") { PersonaEditBlocks().environment(\.look, Look(palette: .farmDark)) }
#Preview("Menu") { HomeView(menuOpen: true).environment(\.look, Look(palette: .farmDark)) }
#Preview("Terminal") { HomeView().environment(\.look, Look(palette: .farmDark, style: .bundled(.terminal))) }
#Preview("Scene") { HomeView().environment(\.look, Look(palette: .farmLight, style: .bundled(.scene))) }
#Preview("Pair") { PairingBlocks(step: .welcome).environment(\.look, Look(palette: .farmDark)) }
