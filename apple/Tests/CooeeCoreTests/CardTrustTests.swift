import CooeeCore
import Testing

internal struct CardTrustTests {
    @Test(arguments: ["secret", "run", "confirm", "permissions"])
    internal func aGuardedCardWithNoPosterIsNotDrawn(_ kind: String) {
        #expect(!CardTrust.draws(kind, from: nil))
        #expect(CardTrust.draws(kind, from: .core))
    }

    @Test internal func onlyCorePostsPermissions() {
        #expect(!CardTrust.draws("permissions", from: .plugin("edgar")))
        #expect(CardTrust.draws("secret", from: .plugin("edgar")) && CardTrust.draws("run", from: .plugin("edgar")))
    }

    @Test internal func anUnguardedCardDrawsFromAnyone() {
        #expect(CardTrust.draws("ask", from: nil) && CardTrust.draws(nil, from: nil))
    }
}
