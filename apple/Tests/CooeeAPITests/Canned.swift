import CooeeAPI
import Foundation
import Synchronization
import Testing

internal struct Answer: Sendable {
    internal var status = 200
    internal var headers: [String: String] = [:]
    internal var chunks: [String]
}

internal final class Canned: URLProtocol, @unchecked Sendable {
    private static let answers = Mutex<[String: [String: [Answer]]]>([:])
    private static let seen = Mutex<[URLRequest]>([])

    override internal static func canInit(with request: URLRequest) -> Bool { true }

    override internal static func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    internal static func register(_ host: String, _ canned: [String: [Answer]]) {
        answers.withLock { $0[host] = canned }
    }

    internal static func requests(_ host: String, _ path: String) -> [URLRequest] {
        seen.withLock { $0.filter { $0.url?.host() == host && $0.url?.path == path } }
    }

    private static func next(_ host: String, _ path: String) -> Answer {
        answers.withLock { canned -> Answer? in
            guard var list = canned[host]?[path], let first = list.first else { return nil }
            if list.count > 1 { list.removeFirst() }
            canned[host]?[path] = list
            return first
        } ?? Answer(status: 404, chunks: [])
    }

    private static func read(_ request: URLRequest) -> URLRequest {
        guard let stream = request.httpBodyStream else { return request }
        var copy = request
        var data = Data()
        var buffer = [UInt8](repeating: 0, count: 4096)
        stream.open()
        while stream.hasBytesAvailable {
            let count = stream.read(&buffer, maxLength: buffer.count)
            guard count > 0 else { break }
            data.append(buffer, count: count)
        }
        copy.httpBody = data
        return copy
    }

    override internal func startLoading() {
        let request = Self.read(request)
        Self.seen.withLock { $0.append(request) }
        guard let url = request.url else { return }
        let answer = Self.next(url.host() ?? "", url.path)
        guard
            let response = HTTPURLResponse(
                url: url, statusCode: answer.status, httpVersion: "HTTP/1.1", headerFields: answer.headers
            )
        else { return }
        client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
        for chunk in answer.chunks { client?.urlProtocol(self, didLoad: Data(chunk.utf8)) }
        client?.urlProtocolDidFinishLoading(self)
    }

    override internal func stopLoading() {}
}

internal struct CannedServer {
    private let host = "\(UUID().uuidString.lowercased()).test"

    internal init(_ answers: [String: [Answer]]) {
        Canned.register(host, answers)
    }

    internal func client(token: String? = "tok", retry: Duration = .zero) -> Client {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [Canned.self]
        let base = URL(string: "http://\(host):4100") ?? URL(filePath: "/")
        return Client(base: base, token: token, configuration: configuration, retry: retry)
    }

    internal func requests(_ path: String) -> [URLRequest] { Canned.requests(host, path) }

    internal func body(_ path: String) -> String? {
        requests(path).first?.httpBody.flatMap { String(bytes: $0, encoding: .utf8) }
    }

    internal func decoded<T: Decodable>(_ path: String, as type: T.Type) throws -> T {
        try JSONDecoder().decode(type, from: try #require(requests(path).first?.httpBody))
    }
}
