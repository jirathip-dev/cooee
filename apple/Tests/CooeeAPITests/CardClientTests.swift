@testable import CooeeAPI
import CooeeCore
import CryptoKit
import Foundation
import Testing

internal struct CardClientTests {
    private struct Sent: Decodable {
        fileprivate let item: Int
        fileprivate let passkey: [String: String]?
    }

    private static func challenge(_ challenge: String, _ ids: [String]) -> Answer {
        let listed = ids.map { "\"\($0)\"" }.joined(separator: ",")
        return Answer(headers: json, chunks: [#"{"challenge":"\#(challenge)","ids":[\#(listed)]}"#])
    }

    private static func bytes(_ text: String?) -> Data {
        let base = (text ?? "").replacing("-", with: "+").replacing("_", with: "/")
        return Data(base64Encoded: base.padding(toLength: (base.count + 3) / 4 * 4, withPad: "=", startingAt: 0))
            ?? Data()
    }

    private static func client(_ field: String?) -> [String: String] {
        (try? JSONDecoder().decode([String: String].self, from: bytes(field))) ?? [:]
    }

    @Test internal func aCardComesAsItsBlockAndItsAnswerPostsTheItemsChoice() async throws {
        let server = CannedServer([
            "/threads/a1b2/messages": [try Fixture.answer("messages.json")],
            "/threads/a1b2/approve": [Answer(headers: json, chunks: ["null"])],
        ])
        let client = server.client()
        let card = try await client.messages("a1b2").items[3]
        #expect(card.block?.list("blocks").first?.list("options").first?.string("value") == "allow")
        try await client.answer("a1b2", item: 4, option: "save", value: "s3cret")
        #expect(
            try server.decoded("/threads/a1b2/approve", as: [String: JSONValue].self)
                == ["item": .number(4), "option": .string("save"), "value": .string("s3cret")]
        )
    }

    @Test internal func approvingCooeesCardRegistersTheDeviceKeyOnceAndSignsEachChallengeWithIt() async throws {
        let signing = P256.Signing.PrivateKey()
        let key = DeviceKey(point: signing.publicKey.x963Representation) {
            try signing.signature(for: $0).derRepresentation
        }
        let server = CannedServer([
            "/passkey/challenge": [Self.challenge("c1", []), Self.challenge("c2", []), Self.challenge("c3", [key.id])],
            "/passkeys": [Answer(chunks: ["passkey saved"])],
            "/threads/a1b2/approve": [Answer(headers: json, chunks: ["null"])],
        ])
        let client = server.client()
        try await client.answer("a1b2", item: 4, option: "approve", value: nil, key: key)
        try await client.answer("a1b2", item: 5, option: "approve", value: nil, key: key)
        let saved = try server.decoded("/passkeys", as: [String: String].self)
        #expect(server.requests("/passkeys").count == 1 && saved["id"] == key.id)
        #expect(Self.bytes(saved["key"]).suffix(65) == signing.publicKey.x963Representation)
        let made = Self.client(saved["client"])
        #expect(made == ["type": "webauthn.create", "challenge": "c1", "origin": "http://\(server.host):4100"])
        let sent = try server.requests("/threads/a1b2/approve").map {
            try JSONDecoder().decode(Sent.self, from: try #require($0.httpBody))
        }
        #expect(sent.map(\.item) == [4, 5] && sent.compactMap(\.passkey).count == 2)
        for (proof, challenge) in zip(sent.compactMap(\.passkey), ["c2", "c3"]) {
            #expect(Self.client(proof["client"])["challenge"] == challenge && proof["id"] == key.id)
            try Self.signed(proof, by: signing.publicKey, for: server.host)
        }
    }

    private static func signed(_ proof: [String: String], by key: P256.Signing.PublicKey, for host: String) throws {
        let (data, auth) = (bytes(proof["client"]), bytes(proof["auth"]))
        #expect(auth.prefix(33) == Data(SHA256.hash(data: Data(host.utf8))) + [0x05])
        let signature = try P256.Signing.ECDSASignature(derRepresentation: bytes(proof["sig"]))
        #expect(key.isValidSignature(signature, for: auth + SHA256.hash(data: data)))
    }
}
