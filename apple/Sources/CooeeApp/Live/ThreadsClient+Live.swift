import ComposableArchitecture
import CooeeAPI
import CooeeFeatures

extension ThreadsClient: DependencyKey {
    public static let liveValue = ThreadsClient(
        home: { server in try await Client(server).home() },
        personas: { server in try await Client(server).personas() },
        looks: { server in try await Client(server).looks() },
        checkTheme: { server, draft in try await Client(server).checkTheme(dark: draft.dark, light: draft.light) },
        saveTheme: { server, name, draft in try await Client(server).saveTheme(name, draft) },
        themeArt: { server, theme, file in try await Client(server).themeArt(theme, file) },
        messages: { server, thread in try await Client(server).messages(thread) },
        frame: { server, thread, name in try await Client(server).frame(thread, name) },
        screen: { server, persona in try await Client(server).screen(persona) },
        takeControl: { server, persona in try await Client(server).takeControl(persona) },
        handBack: { server, persona, note in try await Client(server).handBack(persona, note: note) },
        resume: { server, thread, note in try await Client(server).resumeHandoff(thread, note: note) },
        input: { server, persona, input in try await Client(server).input(persona, input) },
        live: { server, persona in Client(server).live(persona) },
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
        switchAgent: { server, thread, agent in try await Client(server).switchAgent(thread, to: agent) },
        answer: { server, thread, item, option, value in
            try await Client(server).answer(thread, item: item, option: option, value: value)
        },
        start: { server, persona, machine, text, setup in
            try await Client(server).newThread(persona: persona, machine: machine, text: text, setup: setup)
        },
        move: { server, thread, machine in try await Client(server).move(thread, to: machine) },
        retry: { server, thread in try await Client(server).retry(thread) },
        restart: { server, thread in try await Client(server).restart(thread) },
        topics: { server, owner in try await Client(server).topics(of: owner) },
        retitle: { server, thread, title in try await Client(server).retitle(thread, title) },
        archive: { server, thread, archived in try await Client(server).archive(thread, archived) },
        room: { server, title, members in try await Client(server).newRoom(title: title, members: members) },
        seat: { server, room, persona, place, remove in
            try await Client(server).seat(room, persona: persona, place: place, remove: remove)
        }
    )
}
