import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension DeskClient: DependencyKey {
    public static let liveValue = DeskClient { server, thread in try await client(server).typing(thread) }
}
