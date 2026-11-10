import CooeeCore
import Foundation
import Security

public struct Keychain: Sendable {
    private let service: String

    private var query: [CFString: CFTypeRef] {
        [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: service as CFString,
            kSecAttrAccount: "server" as CFString,
        ]
    }

    public init(service: String = "dev.cooee.app") {
        self.service = service
    }

    public func load() -> Server? {
        var found: CFTypeRef?
        var search = query
        search[kSecReturnData] = kCFBooleanTrue
        guard SecItemCopyMatching(search as CFDictionary, &found) == errSecSuccess, let data = found as? Data else {
            return nil
        }
        return try? JSONDecoder().decode(Server.self, from: data)
    }

    @discardableResult
    public func save(_ server: Server) -> Bool {
        clear()
        guard let data = try? JSONEncoder().encode(server) else { return false }
        var item = query
        item[kSecValueData] = data as CFData
        item[kSecAttrAccessible] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        return SecItemAdd(item as CFDictionary, nil) == errSecSuccess
    }

    public func clear() {
        SecItemDelete(query as CFDictionary)
    }
}
