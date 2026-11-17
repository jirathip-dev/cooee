import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension LayoutClient: DependencyKey {
    public static let liveValue = LayoutClient(
        fetch: { server, etag in try await Client(server).layout(etag: etag) },
        events: { server in Client(server).layoutEvents() },
        reset: { server in try await Client(server).resetLayout() }
    )
}
