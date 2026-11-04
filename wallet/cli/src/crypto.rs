//! Keystore encryption primitives.
//!
//! Passphrases are turned into 32-byte keys with argon2id (64 MiB / 3
//! iterations / 1 lane) and keystore payloads are encrypted with
//! XChaCha20-Poly1305 AEAD. The encrypted envelope is
//! `salt(16) || nonce(24) || ciphertext`.
//!
//! Argon2id parameters (64 MiB memory, 3 iterations, 1 lane): 64 MiB memory
//! cost raises the price of offline brute-force attacks while remaining
//! tolerable on a CLI; 3 iterations is the argon2id default and a reasonable
//! work factor; 1 lane matches the single-threaded CLI use case. These values
//! are a defensive starting point and should be revisited before production
//! deployment (see KEYSTORE_ENCRYPTION_PLAN.md risk "Argon2 parameter choice").

use aead::Aead;
use atoll_core::argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use zeroize::Zeroizing;

/// Argon2id memory cost in KiB (64 MiB = 65 536 KiB).
pub const ARGON2_MEMORY_KIB: u32 = 64 * 1024;
/// Argon2id iteration count.
pub const ARGON2_ITERATIONS: u32 = 3;
/// Argon2id lane count.
pub const ARGON2_LANES: u32 = 1;
/// Derived key length in bytes (XChaCha20-Poly1305 key size).
pub const KEY_LEN: usize = 32;
/// Random salt length in bytes.
pub const SALT_LEN: usize = 16;
/// XChaCha20 nonce length in bytes.
pub const NONCE_LEN: usize = 24;

/// Derives a 32-byte encryption key from `passphrase` and `salt` with argon2id.
///
/// The passphrase is wrapped in [`Zeroizing`] so its bytes are scrubbed from
/// memory when the wrapper is dropped (M1: passphrase does not linger in
/// process memory).
pub fn derive_key(passphrase: &str, salt: &[u8; SALT_LEN]) -> Result<[u8; KEY_LEN], String> {
    let argon2 = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(ARGON2_MEMORY_KIB, ARGON2_ITERATIONS, ARGON2_LANES, None)
            .map_err(|e| format!("invalid argon2id parameters: {e}"))?,
    );
    let mut key = [0u8; KEY_LEN];
    let password = Zeroizing::new(passphrase.as_bytes().to_vec());
    argon2
        .hash_password_into(&password, salt, &mut key)
        .map_err(|e| format!("argon2id key derivation failed: {e}"))?;
    Ok(key)
}

/// Encrypts `plaintext` with `passphrase` and returns
/// `salt(16) || nonce(24) || ciphertext`.
pub fn encrypt_keystore(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);

    let key = derive_key(passphrase, &salt)?;
    let cipher = XChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| format!("failed to initialize XChaCha20-Poly1305 cipher: {e}"))?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("XChaCha20-Poly1305 encryption failed: {e}"))?;

    let mut envelope = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    envelope.extend_from_slice(&salt);
    envelope.extend_from_slice(nonce.as_slice());
    envelope.extend_from_slice(&ciphertext);
    Ok(envelope)
}

/// Decrypts a `salt(16) || nonce(24) || ciphertext` envelope with `passphrase`.
pub fn decrypt_keystore(ciphertext: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    if ciphertext.len() < SALT_LEN + NONCE_LEN {
        return Err("ciphertext too short to contain salt and nonce".to_string());
    }

    let (salt, rest) = ciphertext.split_at(SALT_LEN);
    let salt: &[u8; SALT_LEN] = salt
        .try_into()
        .map_err(|_| "invalid salt length".to_string())?;
    let (nonce_bytes, encrypted) = rest.split_at(NONCE_LEN);
    let nonce = XNonce::from_slice(nonce_bytes);

    let key = derive_key(passphrase, salt)?;
    let cipher = XChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| format!("failed to initialize XChaCha20-Poly1305 cipher: {e}"))?;

    cipher
        .decrypt(nonce, encrypted)
        .map_err(|e| format!("decryption failed (wrong passphrase or corrupted keystore): {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_key_is_deterministic() {
        let salt = [1u8; SALT_LEN];
        assert_eq!(
            derive_key("passphrase", &salt).unwrap(),
            derive_key("passphrase", &salt).unwrap()
        );
    }

    #[test]
    fn derive_key_differs_for_different_passphrases() {
        let salt = [1u8; SALT_LEN];
        assert_ne!(
            derive_key("passphrase-a", &salt).unwrap(),
            derive_key("passphrase-b", &salt).unwrap()
        );
    }

    #[test]
    fn derive_key_differs_for_different_salts() {
        let passphrase = "passphrase";
        assert_ne!(
            derive_key(passphrase, &[1u8; SALT_LEN]).unwrap(),
            derive_key(passphrase, &[2u8; SALT_LEN]).unwrap()
        );
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let plaintext = b"secret keystore map";
        let passphrase = "correct-passphrase";
        let encrypted = encrypt_keystore(plaintext, passphrase).unwrap();
        assert_ne!(encrypted.as_slice(), plaintext);
        assert!(encrypted.len() > SALT_LEN + NONCE_LEN);
        let decrypted = decrypt_keystore(&encrypted, passphrase).unwrap();
        assert_eq!(decrypted.as_slice(), plaintext);
    }

    #[test]
    fn wrong_passphrase_fails() {
        let encrypted = encrypt_keystore(b"data", "right-passphrase").unwrap();
        assert!(decrypt_keystore(&encrypted, "wrong-passphrase").is_err());
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let mut encrypted = encrypt_keystore(b"data", "passphrase").unwrap();
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xff;
        assert!(decrypt_keystore(&encrypted, "passphrase").is_err());
    }

    #[test]
    fn tampered_salt_fails() {
        let mut encrypted = encrypt_keystore(b"data", "passphrase").unwrap();
        encrypted[0] ^= 0xff;
        assert!(decrypt_keystore(&encrypted, "passphrase").is_err());
    }

    #[test]
    fn short_ciphertext_fails() {
        assert!(decrypt_keystore(b"too short", "passphrase").is_err());
    }

    #[test]
    fn empty_plaintext_roundtrips() {
        let encrypted = encrypt_keystore(b"", "passphrase").unwrap();
        let decrypted = decrypt_keystore(&encrypted, "passphrase").unwrap();
        assert!(decrypted.is_empty());
    }
}
