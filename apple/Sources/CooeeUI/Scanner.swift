#if os(iOS)
    import SwiftUI
    import VisionKit

    internal struct QRScanner: UIViewControllerRepresentable {
        internal final class Coordinator: NSObject, DataScannerViewControllerDelegate {
            private let found: (String) -> Void
            private var done = false

            internal init(found: @escaping (String) -> Void) {
                self.found = found
            }

            internal func dataScanner(
                _ scanner: DataScannerViewController,
                didAdd items: [RecognizedItem],
                allItems: [RecognizedItem]
            ) {
                for case .barcode(let barcode) in items {
                    guard !done, let link = barcode.payloadStringValue else { continue }
                    done = true
                    scanner.stopScanning()
                    found(link)
                }
            }
        }

        internal let found: (String) -> Void

        internal func makeUIViewController(context: Context) -> DataScannerViewController {
            let scanner = DataScannerViewController(
                recognizedDataTypes: [.barcode(symbologies: [.qr])],
                isHighlightingEnabled: true
            )
            scanner.delegate = context.coordinator
            if DataScannerViewController.isSupported, DataScannerViewController.isAvailable {
                try? scanner.startScanning()
            }
            return scanner
        }

        internal func updateUIViewController(_ scanner: DataScannerViewController, context: Context) {}

        internal func makeCoordinator() -> Coordinator { Coordinator(found: found) }
    }
#endif
