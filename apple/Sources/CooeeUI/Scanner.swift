#if os(iOS)
import SwiftUI
import VisionKit

/// The camera, reading QR codes (VisionKit); the first one found goes to `found` (the server's pairing link).
struct QRScanner: UIViewControllerRepresentable {
    let found: (String) -> Void
    func makeUIViewController(context: Context) -> DataScannerViewController {
        let v = DataScannerViewController(recognizedDataTypes: [.barcode(symbologies: [.qr])], isHighlightingEnabled: true)
        v.delegate = context.coordinator
        if DataScannerViewController.isSupported, DataScannerViewController.isAvailable { try? v.startScanning() }
        return v
    }
    func updateUIViewController(_ v: DataScannerViewController, context: Context) {}
    func makeCoordinator() -> Coordinator { Coordinator(found: found) }

    final class Coordinator: NSObject, DataScannerViewControllerDelegate {
        let found: (String) -> Void
        var done = false
        init(found: @escaping (String) -> Void) { self.found = found }
        func dataScanner(_ s: DataScannerViewController, didAdd items: [RecognizedItem], allItems: [RecognizedItem]) {
            for case .barcode(let b) in items {
                guard !done, let link = b.payloadStringValue else { continue }
                done = true
                s.stopScanning()
                found(link)
            }
        }
    }
}
#endif
