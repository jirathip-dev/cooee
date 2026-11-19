import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

internal let client = Clients()

extension ThreadsClient: DependencyKey {
    public static let liveValue = ThreadsClient(
        home: { server in try await client(server).home() },
        personas: { server in try await client(server).personas() },
        looks: { server in try await client(server).looks() },
        report: { server, report in try await client(server).report(report) },
        checkTheme: { server, draft in try await client(server).checkTheme(dark: draft.dark, light: draft.light) },
        saveTheme: { server, name, draft in try await client(server).saveTheme(name, draft) },
        themeArt: { server, theme, file in try await client(server).themeArt(theme, file) },
        messages: { server, thread in try await client(server).messages(thread) },
        older: { server, thread, oldest in try await client(server).messages(thread, around: oldest).items },
        frame: { server, thread, name in try await client(server).frame(thread, name) },
        file: { server, thread, name in try await client(server).file(thread, name) },
        screen: { server, persona in try await client(server).screen(persona) },
        takeControl: { server, persona in try await client(server).takeControl(persona) },
        handBack: { server, persona, note in try await client(server).handBack(persona, note: note) },
        resume: { server, thread, note in try await client(server).resumeHandoff(thread, note: note) },
        input: { server, persona, input in try await client(server).input(persona, input) },
        live: { server, persona in client(server).live(persona) },
        events: { server, thread, since in client(server).events(thread, since: since) },
        send: { server, thread, text, files, queue in
            try await client(server).send(thread, text: text, files: files, queue: queue)
        },
        steer: { server, thread, randomID in try await client(server).steer(thread, randomID) },
        unqueue: { server, thread, randomID in try await client(server).unqueue(thread, randomID) },
        stop: { server, thread in try await client(server).cancel(thread) },
        read: { server, thread, item in try await client(server).read(thread, upTo: item) },
        model: { server, thread, model, effort in
            try await client(server).model(thread, model: model, effort: effort)
        },
        switchAgent: { server, thread, agent in try await client(server).switchAgent(thread, to: agent) },
        answer: { server, thread, item, option, value in
            try await client(server).answer(thread, item: item, option: option, value: value)
        },
        approve: { server, thread, item, value in
            let key = try DeviceKeychain(server: server.url).key()
            try await client(server).answer(thread, item: item, option: "approve", value: value, key: key)
        },
        start: { server, persona, machine, text, setup in
            try await client(server).newThread(persona: persona, machine: machine, text: text, setup: setup)
        },
        move: { server, thread, machine in try await client(server).move(thread, to: machine) },
        retry: { server, thread in try await client(server).retry(thread) },
        restart: { server, thread in try await client(server).restart(thread) },
        topics: { server, owner in try await client(server).topics(of: owner) },
        retitle: { server, thread, title in try await client(server).retitle(thread, title) },
        archive: { server, thread, archived in try await client(server).archive(thread, archived) },
        room: { server, title, members in try await client(server).newRoom(title: title, members: members) },
        seat: { server, room, persona, place, remove in
            try await client(server).seat(room, persona: persona, place: place, remove: remove)
        }
    )
}
