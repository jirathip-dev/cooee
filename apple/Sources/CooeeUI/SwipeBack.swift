#if os(iOS)
    import SwiftUI
    import UIKit

    internal struct SwipeBack: UIViewControllerRepresentable {
        @MainActor
        internal final class Coordinator: NSObject, UIGestureRecognizerDelegate {
            internal weak var stack: UINavigationController?

            internal func gestureRecognizerShouldBegin(_ gesture: UIGestureRecognizer) -> Bool {
                (stack?.viewControllers.count ?? 0) > 1
            }
        }

        private final class Probe: UIViewController {
            private let coordinator: Coordinator

            internal init(coordinator: Coordinator) {
                self.coordinator = coordinator
                super.init(nibName: nil, bundle: nil)
            }

            @available(*, unavailable)
            internal required init?(coder: NSCoder) { nil }

            override internal func viewDidAppear(_ animated: Bool) {
                super.viewDidAppear(animated)
                coordinator.stack = navigationController
                navigationController?.interactivePopGestureRecognizer?.delegate = coordinator
            }
        }

        internal func makeCoordinator() -> Coordinator { Coordinator() }

        internal func makeUIViewController(context: Context) -> UIViewController {
            Probe(coordinator: context.coordinator)
        }

        internal func updateUIViewController(_ controller: UIViewController, context: Context) {}
    }
#endif
