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

#if os(macOS)
    import AVFoundation
    import SwiftUI

    internal struct MacQRScanner: NSViewRepresentable {
        internal final class Coordinator: NSObject, AVCaptureMetadataOutputObjectsDelegate {
            fileprivate let session = AVCaptureSession()
            private let found: (String) -> Void
            private var done = false

            internal init(found: @escaping (String) -> Void) {
                self.found = found
            }

            internal func metadataOutput(
                _ output: AVCaptureMetadataOutput,
                didOutput objects: [AVMetadataObject],
                from connection: AVCaptureConnection
            ) {
                let codes = objects.compactMap { ($0 as? AVMetadataMachineReadableCodeObject)?.stringValue }
                guard !done, let link = codes.first else { return }
                done = true
                session.stopRunning()
                found(link)
            }
        }

        internal let found: (String) -> Void

        internal func makeNSView(context: Context) -> NSView {
            let (session, output, view) = (context.coordinator.session, AVCaptureMetadataOutput(), NSView())
            if let camera = AVCaptureDevice.default(for: .video), let input = try? AVCaptureDeviceInput(device: camera),
                session.canAddInput(input), session.canAddOutput(output)
            {
                session.addInput(input)
                session.addOutput(output)
                output.setMetadataObjectsDelegate(context.coordinator, queue: .main)
                output.metadataObjectTypes = [.qr]
                session.startRunning()
            }
            let preview = AVCaptureVideoPreviewLayer(session: session)
            preview.videoGravity = .resizeAspectFill
            view.layer = preview
            view.wantsLayer = true
            return view
        }

        internal func updateNSView(_ view: NSView, context: Context) {}

        internal static func dismantleNSView(_ view: NSView, coordinator: Coordinator) {
            coordinator.session.stopRunning()
        }

        internal func makeCoordinator() -> Coordinator { Coordinator(found: found) }
    }
#endif
