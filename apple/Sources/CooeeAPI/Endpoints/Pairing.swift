import CooeeCore
import Foundation

extension Client {
    internal static func token(_ response: HTTPURLResponse) -> String? {
        let cookie = response.value(forHTTPHeaderField: "Set-Cookie") ?? ""
        guard let range = cookie.range(of: "cooee=") else { return nil }
        return String(cookie[range.upperBound...].prefix { $0 != ";" })
    }

    public func pair(code: String) async throws -> String {
        var request = try request("POST", "/pair")
        request.setValue("application/x-www-form-urlencoded", forHTTPHeaderField: "Content-Type")
        request.httpBody = Data("code=\(code.filter(\.isNumber))".utf8)
        let (_, http) = try await send(request, follow: false)
        if let token = Self.token(http) { return token }
        let location = http.value(forHTTPHeaderField: "Location") ?? ""
        throw APIError(status: http.statusCode, text: Self.pairRefusal(location))
    }

    internal static func pairRefusal(_ location: String) -> String {
        if location.contains("bad=expired") { return "That code ran out: get a new one." }
        if location.contains("bad=limit") { return "Too many wrong codes: wait a minute." }
        return "That code is wrong."
    }

    public func askToPair() async throws -> PairAsk {
        try await call("POST", "/pair/request", [String: String]())
    }

    public func claim(_ claim: String) async throws -> PairClaim {
        do {
            let (_, http) = try await send(request("POST", "/pair/claim", json: ["claim": claim]))
            if http.statusCode == 202 { return .waiting }
            return Self.token(http).map(PairClaim.paired) ?? .waiting
        } catch let error as APIError where error.status == 403 {
            return .declined(error.text)
        } catch let error as APIError where error.status == 410 {
            return .gone(error.text)
        }
    }

    public func cancelPair(_ claim: String) async throws {
        try await post("/pair/cancel", ["claim": claim])
    }
}
