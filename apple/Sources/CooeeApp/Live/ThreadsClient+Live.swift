import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension ThreadsClient: DependencyKey {
    public static let liveValue = ThreadsClient(
        home: { server in try await Client(server).home() },
        personas: { server in try await Client(server).personas() },
        looks: { server in try await Client(server).looks() },
        messages: { server, thread in try await Client(server).messages(thread) },
        events: { server, thread, since in Client(server).events(thread, since: since) },
        send: { server, thread, text, files, queue in
            try await Client(server).send(thread, text: text, files: files, queue: queue)
        },
        steer: { server, thread, randomID in try await Client(server).steer(thread, randomID) },
        unqueue: { server, thread, randomID in try await Client(server).unqueue(thread, randomID) },
        stop: { server, thread in try await Client(server).cancel(thread) },
        read: { server, thread, item in try await Client(server).read(thread, upTo: item) },
        model: { server, thread, model, effort in
            try await Client(server).model(thread, model: model, effort: effort)
        },
        answer: { server, thread, item, option, value in
            try await Client(server).answer(thread, item: item, option: option, value: value)
        },
        start: { server, persona, machine, text, setup in
            try await Client(server).newThread(persona: persona, machine: machine, text: text, setup: setup)
        },
        move: { server, thread, machine in try await Client(server).move(thread, to: machine) },
        retry: { server, thread in try await Client(server).retry(thread) },
        room: { server, title, members in try await Client(server).newRoom(title: title, members: members) },
        seat: { server, room, persona, place, remove in
            try await Client(server).seat(room, persona: persona, place: place, remove: remove)
        }
    )
}
