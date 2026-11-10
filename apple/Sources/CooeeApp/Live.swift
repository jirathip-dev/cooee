import ComposableArchitecture
import CooeeAPI
import CooeeCore
import CooeeFeatures
import CooeeUI
import SwiftUI

extension PairingClient: DependencyKey {
    public static let liveValue = PairingClient(
        pair: { url, code in try await Client(base: url).pair(code: code) },
        ask: { url in try await Client(base: url).askToPair() },
        claim: { url, claim in try await Client(base: url).claim(claim) },
        cancel: { url, claim in try await Client(base: url).cancelPair(claim) }
    )
}

extension ThreadsClient: DependencyKey {
    public static let liveValue = ThreadsClient(
        threads: { server in try await Client(server).threads() },
        personas: { server in try await Client(server).personas() },
        messages: { server, thread in try await Client(server).messages(thread) },
        events: { server, thread, since in Client(server).events(thread, since: since) },
        send: { server, thread, text, files, queue in
            try await Client(server).send(thread, text: text, files: files, queue: queue)
        },
        steer: { server, thread, randomID in try await Client(server).steer(thread, randomID) },
        unqueue: { server, thread, randomID in try await Client(server).unqueue(thread, randomID) },
        stop: { server, thread in try await Client(server).cancel(thread) },
        read: { server, thread, item in try await Client(server).read(thread, upTo: item) },
        answer: { server, thread, item, option, value in
            try await Client(server).answer(thread, item: item, option: option, value: value)
        },
        start: { server, persona, text in try await Client(server).newThread(persona: persona, text: text) }
    )
}

extension PersonasClient: DependencyKey {
    public static let liveValue = PersonasClient(
        create: { server, persona in try await Client(server).create(persona) },
        save: { server, persona in try await Client(server).save(persona) },
        memory: { server, persona in try await Client(server).memory(persona) },
        forget: { server, persona in try await Client(server).forget(persona) }
    )
}

extension PushClient: DependencyKey {
    public static let liveValue = PushClient(
        token: { try await PushDelegate.token() },
        register: { server, token, enabled in try await Client(server).push(token, enabled: enabled) }
    )
}

extension SettingsStore: DependencyKey {
    private static let key = "cooee.settings"

    public static let liveValue = SettingsStore(
        load: {
            let data = UserDefaults.standard.data(forKey: key) ?? Data()
            return (try? JSONDecoder().decode(Settings.self, from: data)) ?? Settings()
        },
        save: { settings in UserDefaults.standard.set(try? JSONEncoder().encode(settings), forKey: key) }
    )
}

extension ServerStore: DependencyKey {
    public static let liveValue = ServerStore(
        load: { Keychain().load() },
        save: { server in Keychain().save(server) },
        clear: { Keychain().clear() }
    )
}

extension NearbyClient: DependencyKey {
    public static let liveValue = NearbyClient(browse: { Nearby.browse() })
}

public struct CooeeRoot: View {
    private let store = Store(
        initialState: AppFeature.State(server: ServerStore.liveValue.load(), settings: SettingsStore.liveValue.load())
    ) { AppFeature() }

    public var body: some View {
        AppRoot(store: store)
    }

    public init() {}
}
