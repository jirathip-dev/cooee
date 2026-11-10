import Foundation
import Network
import Observation
import Security

// #143: the live models behind the views: the paired server (Keychain), pairing, Home and a chat.

/// The paired server, one Keychain item (this device only).
public enum Keychain {
    static var query: [String: Any] { [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: "dev.cooee.app",
                                       kSecAttrAccount as String: "server"] }
    public static func load() -> Server? {
        var out: AnyObject?
        var q = query
        q[kSecReturnData as String] = true
        guard SecItemCopyMatching(q as CFDictionary, &out) == errSecSuccess, let data = out as? Data else { return nil }
        return try? JSONDecoder().decode(Server.self, from: data)
    }
    public static func save(_ server: Server) {
        clear()
        var q = query
        q[kSecValueData as String] = try? JSONEncoder().encode(server)
        q[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        SecItemAdd(q as CFDictionary, nil)
    }
    public static func clear() { SecItemDelete(query as CFDictionary) }
}

/// A typed address as the server's base URL (`https://` unless it names `http://`), and the pairing code a QR
/// link carries (`…/pair?code=123456`).
func target(_ typed: String) -> (url: URL, code: String?)? {
    let t = typed.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !t.isEmpty, let c = URLComponents(string: t.contains("://") ? t : "https://\(t)"), let host = c.host,
          !host.isEmpty else { return nil }
    var base = URLComponents()
    base.scheme = c.scheme; base.host = host; base.port = c.port
    return base.url.map { ($0, c.queryItems?.first { $0.name == "code" }?.value) }
}

/// The iPhone's way in: a QR link pairs with its code; an address asks to pair and waits for approval, showing the
/// check word.
@MainActor @Observable public final class Pairing {
    public var step: PhoneStep = .welcome
    public var address = ""
    public var word = ""
    public var until = Date()
    public var error: String?
    public var host = ""
    public internal(set) var server: Server?
    public var personas: [APIPersona] = []
    public let nearby = Nearby()
    var claim: (Client, String)?
    let paired: (Server) -> Void
    public init(paired: @escaping (Server) -> Void) { self.paired = paired }

    public func connect(_ typed: String) async {
        error = nil
        guard let (url, code) = target(typed) else { error = "Type an address, like server.example.com."; return }
        let client = Client(base: url)
        host = url.host() ?? ""
        do {
            if let code {
                done(Server(url: url, token: try await client.pair(code: code)))
                return
            }
            let ask = try await client.askToPair()
            word = ask.word; until = Date().addingTimeInterval(300); claim = (client, ask.claim); step = .waiting
            while let c = claim, c.1 == ask.claim {
                switch try await c.0.claim(c.1) {
                case .paired(let token): done(Server(url: url, token: token)); return
                case .waiting: try await Task.sleep(for: .seconds(2))
                case .declined(let why), .gone(let why): error = why; claim = nil; step = .address
                }
            }
        } catch is CancellationError {
        } catch {
            self.error = error.localizedDescription
        }
    }

    func done(_ s: Server) {
        Keychain.save(s)
        server = s; claim = nil; step = .paired
        Task { personas = (try? await Client(s).personas()) ?? [] }
    }

    public func cancel() {
        if let c = claim { Task { try? await c.0.cancelPair(c.1) } }
        claim = nil; step = .welcome
    }
    public func home() { if let server { paired(server) } }
}

/// cooee servers on the LAN (Bonjour, `_cooee._tcp`), each resolved to its address.
@MainActor @Observable public final class Nearby {
    public private(set) var found: [(name: String, address: String)] = []
    var browser: NWBrowser?
    public func start() {
        guard browser == nil else { return }
        let b = NWBrowser(for: .bonjour(type: "_cooee._tcp", domain: nil), using: .tcp)
        b.browseResultsChangedHandler = { results, _ in
            for r in results { Task { @MainActor in self.resolve(r.endpoint) } }
        }
        b.start(queue: .main)
        browser = b
    }
    public func stop() { browser?.cancel(); browser = nil }
    func resolve(_ e: NWEndpoint) {
        guard case .service(let name, _, _, _) = e, !found.contains(where: { $0.name == name }) else { return }
        let c = NWConnection(to: e, using: .tcp)
        c.stateUpdateHandler = { state in
            guard case .ready = state, case .hostPort(let h, let p) = c.currentPath?.remoteEndpoint else { return }
            let host = "\(h)".split(separator: "%").first.map(String.init) ?? "\(h)"
            Task { @MainActor in
                if !self.found.contains(where: { $0.name == name }) { self.found.append((name, "http://\(host):\(p)")) }
                c.cancel()
            }
        }
        c.start(queue: .main)
    }
}

/// Home's data: the threads and personas, read again every few seconds while it shows.
@MainActor @Observable public final class HomeModel {
    public let client: Client
    public var rows: [ThreadRow] = []
    public var personas: [APIPersona] = []
    public var error: String?
    /// The server refused this device's cookie: it was removed.
    public private(set) var unauthorized = false
    public init(_ client: Client) { self.client = client }
    public func load() async {
        do {
            async let r = client.threads(), p = client.personas()
            (rows, personas) = (try await r.filter { !$0.archived }, try await p)
            error = nil
        } catch {
            unauthorized = (error as? APIError)?.status == 401
            self.error = error.localizedDescription
        }
    }
    public func persona(_ id: String) -> APIPersona? { personas.first { $0.id == id } }
}

/// One open chat: its newest slice, then its event stream from the slice's cursor (a reset loads it again).
@MainActor @Observable public final class ChatModel {
    public let client: Client
    public let id: String
    public private(set) var items: [APIItem] = []
    public private(set) var turn = Turn()
    public private(set) var queue: [Queued] = []
    /// The reply so far, while the turn runs.
    public private(set) var live = ""
    public private(set) var since = Date()
    public var error: String?
    public init(_ client: Client, id: String) { self.client = client; self.id = id }

    public var working: Bool { turn.state == "working" || turn.state == "connecting" }

    /// Runs while the chat shows.
    public func open() async {
        while !Task.isCancelled {
            do {
                let s = try await client.messages(id)
                items = s.items; queue = s.queue; turn = s.turn; since = Date()
                live = working ? s.turn.text ?? "" : ""
                if let last = items.last { try? await client.read(id, upTo: last.id) }
                for try await e in client.events(id, since: s.cursor) { apply(e) }
            } catch is CancellationError {
                return
            } catch {
                self.error = error.localizedDescription
                try? await Task.sleep(for: .seconds(2))
            }
        }
    }

    func apply(_ e: StreamEvent) {
        switch e {
        case .item(let it):
            if let i = items.firstIndex(where: { $0.id == it.id }) { items[i] = it } else { items.append(it) }
            if it.kind == "agent" { live = "" }
        case .text(let t): live += t
        case .state(let t):
            if t.state != turn.state { since = Date() }
            turn = Turn(text: nil, state: t.state, note: t.note)
            if !working { live = "" }
        case .queue(let q): queue = q
        case .reset: break
        }
    }

    public func send(_ text: String, files: [Attachment]) async {
        do { try await client.send(id, text: text, files: files, queue: working) } catch { self.error = error.localizedDescription }
    }
    public func act(_ f: @escaping @Sendable (Client, String) async throws -> Void) {
        Task { do { try await f(client, id) } catch { self.error = error.localizedDescription } }
    }
    public func stop() { act { try await $0.cancel($1) } }
    public func steer(_ q: Queued) { act { try await $0.steer($1, q.random_id) } }
    public func unqueue(_ q: Queued) { act { try await $0.unqueue($1, q.random_id) } }
}

// MARK: The API's shapes as the views draw them

extension LaneState {
    /// A thread's state (`docs/api.md` Row.state) as a lane's.
    init(api s: String) {
        switch s {
        case "working", "connecting": self = .working
        case "blocked": self = .needs
        case "done": self = .done
        case "error", "unconnected": self = .fail
        default: self = .idle
        }
    }
}

extension Persona {
    /// A persona from the server: initials, a tint and coat picked by its id, the state its newest chat is in.
    public init(_ a: APIPersona, state: LaneState = .idle) {
        let words = a.name.split(separator: " ")
        let initials = words.count > 1 ? words.prefix(2).compactMap(\.first).map(String.init).joined() : String(a.name.prefix(2))
        let n = a.id.unicodeScalars.reduce(0) { $0 + Int($1.value) }
        let tints: [Tint] = [.l, .link, .c, .p, .g, .a], coats = ["ch", "gry", "blk", "bay", "wht"]
        self.init(id: a.id, name: a.name, initials: initials.uppercased(), symbol: "person.fill", tint: tints[n % tints.count],
                  coat: coats[n % coats.count], state: state, agent: a.agent, model: a.model, effort: a.effort, folder: a.folder,
                  character: a.character, instructions: a.instructions, plugins: a.plugins)
    }
}

extension HomeModel {
    /// The pinned row: each persona, in the state of its newest chat.
    public var pinned: [Persona] {
        personas.map { p in Persona(p, state: rows.first { $0.persona == p.id }.map { LaneState(api: $0.state) } ?? .idle) }
    }
    /// The chat list, newest first, with each row's persona (a thread without one draws its agent).
    public func list(now: Date = Date()) -> [(Row, Persona)] {
        rows.map { r in
            let state = LaneState(api: r.state)
            let p = persona(r.persona).map { Persona($0, state: state) }
                ?? Persona(APIPersona(id: r.agent, name: r.agent, agent: r.agent, model: "", effort: "", folder: "",
                                      character: "", instructions: "", plugins: []), state: state)
            return (Row(id: r.id, persona: p.id, title: r.title, last: r.last, at: ago(r.at, now: now), unseen: r.unseen,
                        state: state), p)
        }
    }
}

/// `now`, `12m`, `3h`, `2d`.
func ago(_ ms: Int64, now: Date) -> String {
    let s = max(0, Int(now.timeIntervalSince1970) - Int(ms / 1000))
    return s < 60 ? "now" : s < 3600 ? "\(s / 60)m" : s < 86400 ? "\(s / 3600)h" : "\(s / 86400)d"
}

/// A thread's items as the chat draws them: consecutive tool calls folded into one row, agent text as blocks, cards
/// as placeholders (build issue 2), the reply so far last with its caret.
public func chatItems(_ items: [APIItem], live: String) -> [Item] {
    var out: [Item] = []
    for it in items {
        switch it.kind {
        case "you": out.append(.you(id: it.id, text: it.text ?? "", files: it.files ?? []))
        case "agent": out.append(.agent(id: it.id, markdown: blocks(it.text ?? "")))
        case "tool":
            let row = (title: it.title ?? "", status: it.status == "completed" ? "done" : it.status == "failed" ? "failed" : "running")
            if case .tools(let id, let rows) = out.last { out[out.count - 1] = .tools(id: id, rows: rows + [row]) }
            else { out.append(.tools(id: it.id, rows: [row])) }
        case "approval": out.append(.note(id: it.id, text: "A card waits: \(it.tool ?? "an ask"). Answer it on the web for now."))
        default: if let t = it.text, !t.isEmpty { out.append(.note(id: it.id, text: t)) }
        }
    }
    if !live.isEmpty { out.append(.agent(id: Int.max, markdown: blocks(live + " ▍"))) }
    return out
}

/// Agent markdown as blocks: fenced code, runs of `-`/`*` bullets, and paragraphs (inline marks kept).
func blocks(_ md: String) -> [Block] {
    var out: [Block] = [], para: [String] = [], bullets: [String] = [], code: (lang: String, lines: [String])?
    func flush() {
        if !para.isEmpty { out.append(.text(para.joined(separator: "\n"))); para = [] }
        if !bullets.isEmpty { out.append(.bullets(bullets)); bullets = [] }
    }
    for line in md.split(separator: "\n", omittingEmptySubsequences: false).map(String.init) {
        if line.hasPrefix("```") {
            if let c = code { out.append(.code(lang: c.lang, c.lines.joined(separator: "\n"))); code = nil }
            else { flush(); code = (String(line.dropFirst(3)).trimmingCharacters(in: .whitespaces), []) }
        } else if code != nil {
            code!.lines.append(line)
        } else if line.hasPrefix("- ") || line.hasPrefix("* ") {
            if !para.isEmpty { flush() }
            bullets.append(String(line.dropFirst(2)))
        } else if line.trimmingCharacters(in: .whitespaces).isEmpty {
            flush()
        } else {
            if !bullets.isEmpty { flush() }
            para.append(line)
        }
    }
    if let c = code { out.append(.code(lang: c.lang, c.lines.joined(separator: "\n"))) }
    flush()
    return out
}
