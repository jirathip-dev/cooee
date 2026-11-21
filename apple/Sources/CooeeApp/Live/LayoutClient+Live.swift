import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension LayoutClient: DependencyKey {
    public static let liveValue = LayoutClient(
        fetch: { server, etag in try await client(server).layout(etag: etag) },
        events: { server in client(server).layoutEvents() },
        reset: { server in try await client(server).resetLayout() }
    )
}
