import CryptoKit
import Foundation

let env = ProcessInfo.processInfo.environment
func need(_ name: String) -> String {
    guard let value = env[name], !value.isEmpty else { fatalError("\(name) is not set") }
    return value
}
func base64url(_ data: Data) -> String {
    data.base64EncodedString().replacingOccurrences(of: "+", with: "-")
        .replacingOccurrences(of: "/", with: "_").replacingOccurrences(of: "=", with: "")
}
let key = try P256.Signing.PrivateKey(pemRepresentation: String(contentsOfFile: need("ASC_KEY_PATH"), encoding: .utf8))
let now = Int(Date().timeIntervalSince1970)
let header = ["alg": "ES256", "kid": need("ASC_KEY_ID"), "typ": "JWT"]
let claims: [String: Any] = ["iss": need("ASC_ISSUER_ID"), "iat": now, "exp": now + 1200, "aud": "appstoreconnect-v1"]
let input = try [header, claims].map { base64url(try JSONSerialization.data(withJSONObject: $0)) }.joined(separator: ".")
print(input + "." + base64url(try key.signature(for: Data(input.utf8)).rawRepresentation))
