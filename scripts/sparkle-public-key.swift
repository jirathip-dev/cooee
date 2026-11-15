// #333: prints the public EdDSA key (base64) of Sparkle's private key in SPARKLE_ED_KEY: generate_keys exports the
// 32-byte seed; an older export is 96 bytes, the expanded private key then the public key.
import CryptoKit
import Foundation

let secret = ProcessInfo.processInfo.environment["SPARKLE_ED_KEY"] ?? ""
guard let key = Data(base64Encoded: secret.trimmingCharacters(in: .whitespacesAndNewlines)) else {
    FileHandle.standardError.write(Data("SPARKLE_ED_KEY is not base64\n".utf8))
    exit(1)
}
switch key.count {
case 32: print(try Curve25519.Signing.PrivateKey(rawRepresentation: key).publicKey.rawRepresentation.base64EncodedString())
case 96: print(key.suffix(32).base64EncodedString())
default:
    FileHandle.standardError.write(Data("SPARKLE_ED_KEY holds \(key.count) bytes, not a Sparkle EdDSA key\n".utf8))
    exit(1)
}
