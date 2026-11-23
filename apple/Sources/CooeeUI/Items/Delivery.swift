import SwiftUI

internal struct Delivery: View {
    @Environment(\.look) private var look
    internal let failure: String?
    internal let retry: () -> Void

    internal var body: some View {
        let palette = look.palette
        VStack(alignment: .trailing, spacing: 2) {
            if let failure {
                Button("Not delivered · Retry", action: retry).buttonStyle(.plain).ui(12, .semibold, look)
                    .foregroundStyle(palette.red)
                Text(failure).ui(12, .regular, look).foregroundStyle(palette.mute)
            } else {
                Text("Sending…").ui(12, .regular, look).foregroundStyle(palette.mute)
            }
        }
        .frame(maxWidth: .infinity, alignment: .trailing)
    }
}
