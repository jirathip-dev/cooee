import CooeeCore
import Foundation

extension Client {
    public func looks() async throws -> Looks { try await call("GET", "/looks") }

    public func checkTheme(dark: [String: String], light: [String: String]) async throws -> [String: GateReport] {
        try await call("POST", "/themes/check", ["dark": dark, "light": light])
    }

    public func saveTheme(_ name: String, _ draft: ThemeDraft) async throws {
        _ = try await send(request("PUT", "/themes/\(name)", json: draft))
    }

    public func themeArt(_ theme: String, _ file: String) async throws -> String {
        String(bytes: try await send(request("GET", "/look/\(theme)/\(file)")).0, encoding: .utf8) ?? ""
    }
}
