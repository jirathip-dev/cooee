import SwiftUI

internal struct Copyable: ViewModifier {
    @Environment(\.message) private var message
    @State private var selecting = false
    internal let text: String
    internal var id: Int?

    internal func body(content: Content) -> some View {
        content
            .contextMenu {
                Button("Copy", systemImage: "doc.on.doc") { copy(text) }
                if let id, let quote = message.quote {
                    Button("Copy as markdown", systemImage: "chevron.left.forwardslash.chevron.right") {
                        copy(message.source(id) ?? text)
                    }
                    Button("Quote", systemImage: "text.quote") { quote(id) }
                }
                Button("Select text", systemImage: "selection.pin.in.out") { selecting = true }
            }
            .sheet(isPresented: $selecting) { SelectSheet(text: text) }
    }
}

internal struct SelectSheet: View {
    @Environment(\.dismiss) private var dismiss
    internal let text: String

    internal var body: some View {
        NavigationStack {
            SelectableText(text: text).padding(.horizontal, 12)
                .navigationTitle("Select text")
                .toolbar {
                    ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } }
                }
        }
    }
}

#if os(iOS)
    internal struct SelectableText: UIViewRepresentable {
        internal let text: String

        internal func makeUIView(context: Context) -> UITextView {
            let view = UITextView()
            view.isEditable = false
            view.backgroundColor = .clear
            view.font = .preferredFont(forTextStyle: .body)
            view.adjustsFontForContentSizeCategory = true
            return view
        }

        internal func updateUIView(_ view: UITextView, context: Context) {
            view.text = text
        }
    }
#else
    internal struct SelectableText: View {
        internal let text: String

        internal var body: some View {
            ScrollView {
                Text(text).textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }
#endif
