import ComposableArchitecture
import CooeeAPI
import CooeeFeatures
import Foundation

private let copies = KeptScreens(
    folder: URL.applicationSupportDirectory.appending(path: "kept", directoryHint: .isDirectory))

extension KeptCopies: DependencyKey {
    public static let liveValue = KeptCopies(
        home: { server in copies.load("\(server.url)|home") },
        keepHome: { server, home in copies.save("\(server.url)|home", home) },
        thread: { server, thread in copies.load("\(server.url)|thread|\(thread)") },
        keepThread: { server, thread, slice in copies.save("\(server.url)|thread|\(thread)", slice) }
    )
}
