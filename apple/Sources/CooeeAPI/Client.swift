import CooeeCore
import Foundation

public struct APIError: Error, LocalizedError, Equatable {
    public let status: Int
    public let text: String

    public var errorDescription: String? { text.isEmpty ? "The server answered \(status)." : text }

    public init(status: Int, text: String) {
        self.status = status
        self.text = text
    }
}

private final class NoRedirect: NSObject, URLSessionTaskDelegate {
    fileprivate func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        willPerformHTTPRedirection response: HTTPURLResponse,
        newRequest request: URLRequest
    ) async -> URLRequest? { nil }
}

public final class Client: Sendable {
    public let base: URL
    internal let token: String?
    internal let session: URLSession
    internal let retry: Duration

    public init(
        base: URL,
        token: String? = nil,
        configuration: URLSessionConfiguration = .ephemeral,
        retry: Duration = .seconds(2)
    ) {
        self.base = base
        self.token = token
        self.retry = retry
        configuration.httpShouldSetCookies = false
        session = URLSession(configuration: configuration)
    }

    public convenience init(_ server: Server) {
        self.init(base: server.url, token: server.token)
    }

    internal func request(_ method: String, _ path: String, json: (any Encodable)? = nil) throws -> URLRequest {
        guard let url = URL(string: path, relativeTo: base) else { throw APIError(status: 400, text: "Bad path \(path).") }
        var request = URLRequest(url: url)
        request.httpMethod = method
        if let token { request.setValue("cooee=\(token)", forHTTPHeaderField: "Cookie") }
        if let json {
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
            request.httpBody = try JSONEncoder().encode(json)
        }
        return request
    }

    internal static func http(_ response: URLResponse) throws -> HTTPURLResponse {
        guard let http = response as? HTTPURLResponse else { throw APIError(status: 0, text: "Not an HTTP answer.") }
        return http
    }

    internal func send(_ request: URLRequest, follow: Bool = true) async throws -> (Data, HTTPURLResponse) {
        let (data, response) = try await session.data(for: request, delegate: follow ? nil : NoRedirect())
        let http = try Self.http(response)
        if http.statusCode >= 400 {
            let own = http.value(forHTTPHeaderField: "Cooee-Error") == "1"
            throw APIError(status: http.statusCode, text: own ? String(decoding: data, as: UTF8.self) : "")
        }
        return (data, http)
    }

    internal func call<T: Decodable>(_ method: String, _ path: String, _ body: (any Encodable)? = nil) async throws -> T {
        try JSONDecoder().decode(T.self, from: try await send(request(method, path, json: body)).0)
    }

    internal func post(_ path: String, _ body: some Encodable) async throws {
        _ = try await send(request("POST", path, json: body))
    }
}
