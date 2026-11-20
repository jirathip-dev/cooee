use std::io;

use aes_gcm::{
    Aes128Gcm, KeyInit, Nonce,
    aead::{Aead, generic_array::GenericArray},
};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD as B64},
};
use cooee_core::random;
use hkdf::Hkdf;
use p256::{
    PublicKey, SecretKey,
    ecdsa::{Signature, SigningKey, signature::Signer},
    elliptic_curve::sec1::ToSec1Point,
};
use sha2::Sha256;

pub(super) fn b64(s: &str) -> io::Result<Vec<u8>> {
    B64.decode(s.trim_end_matches('='))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
}

pub(super) fn new_key() -> io::Result<SecretKey> {
    loop {
        if let Ok(k) = SecretKey::from_slice(&random(32)?) {
            return Ok(k);
        }
    }
}

pub(super) fn public(k: &SecretKey) -> Vec<u8> {
    k.public_key().to_sec1_point(false).as_bytes().to_vec()
}

/// RFC 8291 aes128gcm body for one subscription: a single record, `salt` and `eph`'s public key in its header.
///
/// # Errors
/// When `p256dh` isn't a P-256 point.
pub fn encrypt(
    p256dh: &[u8],
    auth: &[u8],
    plain: &[u8],
    eph: &SecretKey,
    salt: &[u8; 16],
) -> io::Result<Vec<u8>> {
    let bad = |_| io::Error::new(io::ErrorKind::InvalidInput, "bad push key");
    let ua = PublicKey::from_sec1_bytes(p256dh).map_err(bad)?;
    let shared = p256::ecdh::diffie_hellman(eph.to_nonzero_scalar(), ua.as_affine());
    let as_pub = public(eph);
    let info = [b"WebPush: info\0".as_slice(), p256dh, &as_pub].concat();
    let hk = |salt: &[u8], ikm: &[u8], info: &[u8], out: &mut [u8]| {
        Hkdf::<Sha256>::new(Some(salt), ikm)
            .expand(info, out)
            .map_err(|_| io::Error::other("hkdf"))
    };
    let (mut ikm, mut cek, mut nonce) = ([0u8; 32], [0u8; 16], [0u8; 12]);
    hk(auth, shared.raw_secret_bytes().as_slice(), &info, &mut ikm)?;
    hk(salt, &ikm, b"Content-Encoding: aes128gcm\0", &mut cek)?;
    hk(salt, &ikm, b"Content-Encoding: nonce\0", &mut nonce)?;
    let ct = Aes128Gcm::new(GenericArray::from_slice(&cek))
        .encrypt(Nonce::from_slice(&nonce), [plain, &[2]].concat().as_slice())
        .map_err(|_| io::Error::other("aes-gcm"))?;
    Ok([salt.as_slice(), &4096u32.to_be_bytes(), &[65], &as_pub, &ct].concat())
}

fn es256(k: &SecretKey, header: &serde_json::Value, claims: &serde_json::Value) -> String {
    let msg = format!(
        "{}.{}",
        B64.encode(header.to_string()),
        B64.encode(claims.to_string())
    );
    let sig: Signature = SigningKey::from(k).sign(msg.as_bytes());
    format!("{msg}.{}", B64.encode(sig.to_bytes()))
}

/// The ES256 VAPID JWT for `endpoint`'s origin, valid 12 h from `now` (Unix seconds).
#[must_use]
pub fn jwt(k: &SecretKey, endpoint: &str, now: u64) -> String {
    let aud = endpoint
        .splitn(4, '/')
        .take(3)
        .collect::<Vec<_>>()
        .join("/");
    let sub = "https://github.com/jirathip-dev/cooee";
    let claims = serde_json::json!({"aud": aud, "exp": now + 12 * 3600, "sub": sub});
    es256(
        k,
        &serde_json::json!({"typ": "JWT", "alg": "ES256"}),
        &claims,
    )
}

/// The APNs provider JWT: key `key_id` of team `team_id`, issued at `now`.
#[must_use]
pub fn apns_jwt(k: &SecretKey, key_id: &str, team_id: &str, now: u64) -> String {
    let header = serde_json::json!({"alg": "ES256", "kid": key_id});
    es256(k, &header, &serde_json::json!({"iss": team_id, "iat": now}))
}

pub(super) fn p8_key(pem: &str) -> Option<SecretKey> {
    let b64: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
    let der = STANDARD.decode(b64.trim()).ok()?;
    let at = der.windows(5).position(|w| w == [2, 1, 1, 4, 32])? + 5;
    SecretKey::from_slice(der.get(at..at + 32)?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_matches_rfc_8291_vector() {
        let d = |s: &str| B64.decode(s).unwrap();
        let eph = SecretKey::from_slice(&d("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw")).unwrap();
        let salt: [u8; 16] = d("DGv6ra1nlYgDCS1FRnbzlw").try_into().unwrap();
        let ua = d(
            "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4",
        );
        let body = encrypt(
            &ua,
            &d("BTBZMqHH6r4Tts7J_aSIgg"),
            b"When I grow up, I want to be a watermelon",
            &eph,
            &salt,
        )
        .unwrap();
        assert_eq!(
            B64.encode(body),
            "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPTpK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN"
        );
    }

    #[test]
    fn jwt_is_signed_by_the_key_for_the_endpoint_origin() {
        use p256::ecdsa::{VerifyingKey, signature::Verifier};
        let k = new_key().unwrap();
        let t = jwt(&k, "https://push.example.com/send/abc", 1000);
        let (msg, sig) = t.rsplit_once('.').unwrap();
        let sig = Signature::from_slice(&B64.decode(sig).unwrap()).unwrap();
        VerifyingKey::from(&SigningKey::from(&k))
            .verify(msg.as_bytes(), &sig)
            .unwrap();
        let claims = B64.decode(msg.split('.').nth(1).unwrap()).unwrap();
        let claims: serde_json::Value = serde_json::from_slice(&claims).unwrap();
        assert_eq!(claims["aud"], "https://push.example.com");
        assert_eq!(claims["exp"], 1000 + 12 * 3600);
    }
}
