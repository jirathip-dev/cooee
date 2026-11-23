import CryptoKit
import Foundation

/// A P-256 key that proves this device is the owner: its public point (X9.63) and a signer of a message.
public struct DeviceKey: Sendable {
    private static let spki = Data([
        0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
        0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
    ])

    public let point: Data
    private let signer: @Sendable (Data) throws -> Data

    public var id: String { Data(SHA256.hash(data: point)).base64URL }

    internal var key: String { (Self.spki + point).base64URL }

    public init(point: Data, sign: @escaping @Sendable (Data) throws -> Data) {
        self.point = point
        signer = sign
    }

    internal func sign(_ message: Data) throws -> Data { try signer(message) }
}

internal struct PasskeyChallenge: Decodable {
    internal let challenge: String
    internal let ids: [String]
}

internal struct Passkey: Encodable {
    internal let id: String
    internal let client: String
    internal let auth: String
    internal var key: String?
    internal var sig: String?
}

extension Data {
    internal var base64URL: String {
        base64EncodedString().replacing("+", with: "-").replacing("/", with: "_").replacing("=", with: "")
    }
}

extension Client {
    private func ceremony(_ kind: String, _ challenge: String) throws -> (client: Data, auth: Data) {
        let parts = URLComponents(url: base, resolvingAgainstBaseURL: true)
        let host = parts?.host ?? ""
        let origin = "\(parts?.scheme ?? "https")://\(host)" + (parts?.port.map { ":\($0)" } ?? "")
        let encoder = JSONEncoder()
        encoder.outputFormatting = .withoutEscapingSlashes
        let client = try encoder.encode(["type": kind, "challenge": challenge, "origin": origin])
        return (client, Data(SHA256.hash(data: Data(host.utf8))) + [0x05, 0, 0, 0, 0])
    }

    internal func prove(_ key: DeviceKey) async throws -> Passkey {
        var asked: PasskeyChallenge = try await call("POST", "/passkey/challenge", [String: String]())
        if !asked.ids.contains(key.id) {
            let made = try ceremony("webauthn.create", asked.challenge)
            let saved = Passkey(
                id: key.id, client: made.client.base64URL, auth: made.auth.base64URL, key: key.key)
            try await post("/passkeys", saved)
            asked = try await call("POST", "/passkey/challenge", [String: String]())
        }
        let (client, auth) = try ceremony("webauthn.get", asked.challenge)
        let sig = try key.sign(auth + SHA256.hash(data: client))
        return Passkey(id: key.id, client: client.base64URL, auth: auth.base64URL, sig: sig.base64URL)
    }
}
