import CooeeAPI
import Foundation
import Security

internal struct DeviceKeychain: Sendable {
    private let tag: Data

    private var query: [CFString: CFTypeRef] {
        [
            kSecClass: kSecClassKey,
            kSecAttrKeyType: kSecAttrKeyTypeECSECPrimeRandom,
            kSecAttrApplicationTag: tag as CFData,
            kSecUseDataProtectionKeychain: kCFBooleanTrue,
            kSecReturnRef: kCFBooleanTrue,
        ]
    }

    internal init(server: URL) {
        tag = Data("dev.cooee.device.\(server.absoluteString)".utf8)
    }

    internal func key() throws -> DeviceKey {
        let refused = APIError(status: 0, text: "This device can't keep a key to approve with.")
        guard let key = stored() ?? made(inEnclave: true) ?? made(inEnclave: false),
            let point = SecKeyCopyPublicKey(key).flatMap({ SecKeyCopyExternalRepresentation($0, nil) })
        else { throw refused }
        return DeviceKey(point: point as Data) { message in
            guard let signing = stored(),
                let sig = SecKeyCreateSignature(signing, .ecdsaSignatureMessageX962SHA256, message as CFData, nil)
            else {
                throw APIError(status: 0, text: "Not approved: it wasn't confirmed it's you.")
            }
            return sig as Data
        }
    }

    private func stored() -> SecKey? {
        var found: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &found) == errSecSuccess, let found,
            CFGetTypeID(found) == SecKeyGetTypeID()
        else { return nil }
        return unsafeDowncast(found, to: SecKey.self)
    }

    private func made(inEnclave: Bool) -> SecKey? {
        let flags: SecAccessControlCreateFlags = inEnclave ? [.privateKeyUsage, .userPresence] : .userPresence
        let when = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        guard let access = SecAccessControlCreateWithFlags(nil, when, flags, nil) else { return nil }
        let own: [CFString: CFTypeRef] = [
            kSecAttrIsPermanent: kCFBooleanTrue, kSecAttrApplicationTag: tag as CFData, kSecAttrAccessControl: access,
        ]
        var attributes: [CFString: CFTypeRef] = [
            kSecAttrKeyType: kSecAttrKeyTypeECSECPrimeRandom, kSecAttrKeySizeInBits: 256 as CFNumber,
            kSecUseDataProtectionKeychain: kCFBooleanTrue, kSecPrivateKeyAttrs: own as CFDictionary,
        ]
        if inEnclave { attributes[kSecAttrTokenID] = kSecAttrTokenIDSecureEnclave }
        return SecKeyCreateRandomKey(attributes as CFDictionary, nil)
    }
}
