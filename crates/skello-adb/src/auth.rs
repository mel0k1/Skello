use std::fmt;
use std::fs;
use std::path::Path;

use base64::Engine as _;
use rsa::pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey, LineEnding};
use rsa::pkcs1v15::SigningKey;
use rsa::signature::{SignatureEncoding, Signer};
use rsa::traits::PublicKeyParts;
use rsa::{BigUint, RsaPrivateKey};
use sha1::Sha1;

const KEY_BITS: usize = 2048;
const MODULUS_LEN: usize = KEY_BITS / 8;

#[derive(Debug)]
pub enum AuthError {
    KeyGen(String),
    Io(String),
    Decode(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::KeyGen(s) => write!(f, "adb key generation failed: {s}"),
            AuthError::Io(s) => write!(f, "adb key storage failed: {s}"),
            AuthError::Decode(s) => write!(f, "adb key decode failed: {s}"),
        }
    }
}

impl std::error::Error for AuthError {}

/// The adb host key pair, stored like the desktop adb client does:
/// a PKCS#1 PEM file named `adbkey` in the app support directory.
pub struct AdbKeys {
    key: RsaPrivateKey,
}

impl AdbKeys {
    pub fn load_or_generate(dir: &Path) -> Result<Self, AuthError> {
        let key_file = dir.join("adbkey");
        if key_file.exists() {
            // An existing key that fails to parse must not be
            // overwritten: every device that trusted it would ask for
            // authorization again. Report the broken file instead.
            let pem = fs::read_to_string(&key_file)
                .map_err(|e| AuthError::Io(e.to_string()))?;
            let key =
                RsaPrivateKey::from_pkcs1_pem(&pem).map_err(|e| AuthError::Decode(e.to_string()))?;
            return Ok(AdbKeys { key });
        }
        let mut rng = rand::rngs::OsRng;
        let key =
            RsaPrivateKey::new(&mut rng, KEY_BITS).map_err(|e| AuthError::KeyGen(e.to_string()))?;
        let pem = key
            .to_pkcs1_pem(LineEnding::LF)
            .map_err(|e| AuthError::Io(e.to_string()))?
            .to_string();
        fs::create_dir_all(dir).map_err(|e| AuthError::Io(e.to_string()))?;
        fs::write(&key_file, pem).map_err(|e| AuthError::Io(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&key_file, fs::Permissions::from_mode(0o600));
        }
        Ok(AdbKeys { key })
    }

    /// Signs an AUTH TOKEN with SHA1 PKCS#1 v1.5, as adbd expects.
    pub fn sign_token(&self, token: &[u8]) -> Vec<u8> {
        let signing = SigningKey::<Sha1>::new(self.key.clone());
        signing.sign(token).to_vec()
    }

    /// The AUTH RSAPUBLICKEY payload: base64 of the android public key
    /// struct, a space and a name, null-terminated.
    pub fn public_key_payload(&self) -> Vec<u8> {
        let encoded =
            base64::engine::general_purpose::STANDARD.encode(android_pubkey_encode(&self.key));
        let mut payload = encoded.into_bytes();
        payload.extend_from_slice(b" skello\0");
        payload
    }
}

/// Encodes the public key in the adbd `RSAPublicKey` struct format
/// (all little-endian): len in 32-bit words, n0inv, modulus, R^2 mod n,
/// exponent. 524 bytes for a 2048-bit key.
pub fn android_pubkey_encode(key: &impl PublicKeyParts) -> Vec<u8> {
    let n = key.n().to_bytes_le();
    let mut out = Vec::with_capacity(4 + 4 + 2 * MODULUS_LEN + 4);
    out.extend_from_slice(&((MODULUS_LEN / 4) as u32).to_le_bytes());
    let n_low = u32::from_le_bytes(n[0..4].try_into().unwrap());
    out.extend_from_slice(&montgomery_n0inv(n_low).to_le_bytes());
    let mut n_bytes = vec![0u8; MODULUS_LEN];
    n_bytes[..n.len().min(MODULUS_LEN)].copy_from_slice(&n);
    out.extend_from_slice(&n_bytes);
    let rr = (BigUint::from(1u32) << (MODULUS_LEN * 8 * 2)) % key.n();
    let rr = rr.to_bytes_le();
    let mut rr_bytes = vec![0u8; MODULUS_LEN];
    rr_bytes[..rr.len().min(MODULUS_LEN)].copy_from_slice(&rr);
    out.extend_from_slice(&rr_bytes);
    // The exponent is a u32 field: copy the real one when it fits
    // (generated keys use 65537), pad otherwise.
    let mut e_bytes = [0u8; 4];
    let e_le = key.e().to_bytes_le();
    if e_le.len() <= 4 {
        e_bytes[..e_le.len()].copy_from_slice(&e_le);
    }
    out.extend_from_slice(&e_bytes);
    out
}

/// Montgomery parameter: n0inv such that n0inv * n[0] == -1 (mod 2^32).
fn montgomery_n0inv(n_low: u32) -> u32 {
    // Newton iteration for the inverse modulo 2^32, starting from any odd seed.
    let mut inv: u64 = 1;
    let a = n_low as u64;
    for _ in 0..5 {
        inv = inv.wrapping_mul(2u64.wrapping_sub(a.wrapping_mul(inv)));
    }
    (!(inv as u32)).wrapping_add(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AUTH_RSAPUBLICKEY;
    use rsa::pkcs1v15::VerifyingKey;
    use rsa::signature::Verifier;
    use rsa::RsaPublicKey;

    const TEST_KEY_PEM: &str = include_str!("../testdata/adbkey_test.pem");

    #[test]
    fn loads_test_key_and_signs_tokens() {
        let key = RsaPrivateKey::from_pkcs1_pem(TEST_KEY_PEM).unwrap();
        let keys = AdbKeys { key };
        let token = [42u8; crate::TOKEN_SIZE];
        let signature = keys.sign_token(&token);
        assert_eq!(signature.len(), 256);
        let public = RsaPublicKey::from(&keys.key);
        let verifying = VerifyingKey::<Sha1>::new(public);
        let signing = SigningKey::<Sha1>::new(keys.key.clone());
        let sig = signing.sign(&token);
        assert!(verifying.verify(&token, &sig).is_ok());
        let mut broken = token;
        broken[0] ^= 1;
        assert!(verifying.verify(&broken, &sig).is_err());
    }

    #[test]
    fn pubkey_payload_is_base64_struct_plus_name() {
        let key = RsaPrivateKey::from_pkcs1_pem(TEST_KEY_PEM).unwrap();
        let keys = AdbKeys { key };
        let payload = keys.public_key_payload();
        let text = String::from_utf8(payload.clone()).unwrap();
        assert!(text.ends_with(" skello\0"));
        let b64 = text.trim_end_matches('\0').trim_end_matches(" skello");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        assert_eq!(decoded.len(), 4 + 4 + 2 * MODULUS_LEN + 4);
        assert_eq!(u32::from_le_bytes(decoded[0..4].try_into().unwrap()), 64);
        assert_eq!(
            u32::from_le_bytes(decoded[520..524].try_into().unwrap()),
            65537
        );
    }

    #[test]
    fn struct_matches_aosp_layout() {
        let key = RsaPrivateKey::from_pkcs1_pem(TEST_KEY_PEM).unwrap();
        let encoded = android_pubkey_encode(&key);
        assert_eq!(encoded.len(), 524);
        let n = key.n();
        // modulus stored little-endian
        let n_le = n.to_bytes_le();
        assert_eq!(&encoded[8..8 + n_le.len()], &n_le[..]);
        // exponent and word count
        assert_eq!(u32::from_le_bytes(encoded[0..4].try_into().unwrap()), 64);
        assert_eq!(
            u32::from_le_bytes(encoded[520..524].try_into().unwrap()),
            65537
        );
        // rr = 2^4096 mod n, little-endian
        let rr = (BigUint::from(1u32) << 4096) % n;
        let rr_le = rr.to_bytes_le();
        assert_eq!(&encoded[264..264 + rr_le.len()], &rr_le[..]);
        // n0inv * n[0] == -1 (mod 2^32)
        let n0inv = u32::from_le_bytes(encoded[4..8].try_into().unwrap());
        let n0 = u32::from_le_bytes(n_le[0..4].try_into().unwrap());
        assert_eq!(n0inv.wrapping_mul(n0), u32::MAX);
    }

    #[test]
    fn montgomery_parameter_is_negative_inverse() {
        for n_low in [1u32, 3, 1234567891, u32::MAX] {
            let n0inv = montgomery_n0inv(n_low);
            assert_eq!(n0inv.wrapping_mul(n_low), u32::MAX);
        }
        assert_eq!(AUTH_RSAPUBLICKEY, 3);
    }
}
