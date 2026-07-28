//! AES-256-GCM encryption for BYOK credentials at rest.
//!
//! One static master key per deployment, from `FLUVIOME_CREDENTIAL_KEY` (32
//! raw bytes, base64-encoded — e.g. `openssl rand -base64 32`). No key
//! rotation/KMS in v1. Missing/malformed key is NOT a boot failure for the
//! caller (database-server) — see `CredentialKey::from_base64`'s callers,
//! which should log and continue with `None`, only erroring when a BYOK
//! operation is actually attempted.

// `aes_gcm::Nonce<T>` (re-exported at the crate root) is generic over the
// *size* directly (`GenericArray<u8, T>`) — the wrong shape here. The one we
// want is `aead::Nonce<A>` (`GenericArray<u8, <A as AeadCore>::NonceSize>`),
// generic over the cipher type, matching `Key<Aes256Gcm>`'s own shape.
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, Nonce, OsRng},
    Aes256Gcm, Key,
};
use base64::Engine;

pub const KEY_LEN:   usize = 32;
pub const NONCE_LEN: usize = 12;

#[derive(Clone)]
pub struct CredentialKey(Key<Aes256Gcm>);

impl CredentialKey {
    pub fn from_base64(s: &str) -> anyhow::Result<Self> {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(s.trim())
            .map_err(|e| anyhow::anyhow!("FLUVIOME_CREDENTIAL_KEY is not valid base64: {e}"))?;
        if raw.len() != KEY_LEN {
            anyhow::bail!(
                "FLUVIOME_CREDENTIAL_KEY must decode to {KEY_LEN} bytes, got {}",
                raw.len()
            );
        }
        Ok(Self(*Key::<Aes256Gcm>::from_slice(&raw)))
    }
}

/// Encrypts `plaintext`, returning `nonce (12B) || ciphertext`.
pub fn encrypt(key: &CredentialKey, plaintext: &str) -> anyhow::Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(&key.0);
    let nonce  = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("encryption failed: {e}"))?;

    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypts a blob produced by [`encrypt`] (`nonce (12B) || ciphertext`).
pub fn decrypt(key: &CredentialKey, blob: &[u8]) -> anyhow::Result<String> {
    if blob.len() < NONCE_LEN {
        anyhow::bail!("ciphertext blob too short to contain a nonce");
    }
    let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
    let nonce  = Nonce::<Aes256Gcm>::from_slice(nonce_bytes);
    let cipher = Aes256Gcm::new(&key.0);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("decryption failed (wrong key or corrupted data): {e}"))?;

    String::from_utf8(plaintext)
        .map_err(|e| anyhow::anyhow!("decrypted credential is not valid UTF-8: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let raw = [7u8; KEY_LEN];
        let key = CredentialKey::from_base64(
            &base64::engine::general_purpose::STANDARD.encode(raw)
        ).unwrap();

        let blob = encrypt(&key, "sk-ant-super-secret").unwrap();
        assert_ne!(blob, b"sk-ant-super-secret");
        assert_eq!(decrypt(&key, &blob).unwrap(), "sk-ant-super-secret");
    }

    #[test]
    fn rejects_wrong_key() {
        let key1 = CredentialKey::from_base64(
            &base64::engine::general_purpose::STANDARD.encode([1u8; KEY_LEN])
        ).unwrap();
        let key2 = CredentialKey::from_base64(
            &base64::engine::general_purpose::STANDARD.encode([2u8; KEY_LEN])
        ).unwrap();

        let blob = encrypt(&key1, "secret").unwrap();
        assert!(decrypt(&key2, &blob).is_err());
    }
}
