//! A fake platform authenticator for Face ID approvals (#56, #261).

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use p256::{
    SecretKey,
    ecdsa::{Signature, SigningKey, signature::Signer},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::Cooee;

/// A fake platform authenticator, as in tests/pairing.rs: one P-256 passkey signing for this server's origin.
pub struct Authenticator {
    key: SigningKey,
    id: String,
    host: String,
}

impl Authenticator {
    pub fn new(c: &Cooee) -> Self {
        Self {
            key: SigningKey::from(SecretKey::from_slice(&[7; 32]).unwrap()),
            id: B64.encode([7; 16]),
            host: c.addr.clone(),
        }
    }

    fn signed(&self, kind: &str, c: &Cooee) -> (Vec<u8>, Vec<u8>, String) {
        let (_, body) = c.raw(
            "POST",
            "/passkey/challenge",
            &[("Content-Type", "application/json")],
            "{}",
        );
        let challenge = serde_json::from_str::<Value>(&body).unwrap()["challenge"].clone();
        let client = json!({ "type": kind, "challenge": challenge, "origin": format!("http://{}", self.host) });
        let client = client.to_string().into_bytes();
        let mut auth = Sha256::digest(self.host.split(':').next().unwrap()).to_vec();
        auth.extend([0x05, 0, 0, 0, 1]);
        let mut msg = auth.clone();
        msg.extend(Sha256::digest(&client));
        let sig: Signature = self.key.sign(&msg);
        (auth, client, B64.encode(sig.to_der().as_bytes()))
    }

    /// Saves the passkey for the paired device.
    pub fn save(&self, c: &Cooee) {
        let (auth, client, _) = self.signed("webauthn.create", c);
        let mut spki = vec![
            0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01,
        ];
        spki.extend([
            0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
        ]);
        spki.extend(self.key.verifying_key().to_sec1_point(false).as_bytes());
        let key = json!({ "id": self.id, "key": B64.encode(spki), "client": B64.encode(client), "auth": B64.encode(auth) });
        assert_eq!(c.post("/passkeys", &key).0, 200);
    }

    /// `navigator.credentials.get` after Face ID.
    pub fn get(&self, c: &Cooee) -> Value {
        let (auth, client, sig) = self.signed("webauthn.get", c);
        json!({ "id": self.id, "client": B64.encode(client), "auth": B64.encode(auth), "sig": sig })
    }
}
