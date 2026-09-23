//! The conductor `db.key` format.
//!
//! This is a faithful port of `holochain_data 0.7.0`'s `DbKey::load` /
//! `DbKey::generate` / `DbKey::apply_pragmas`
//! (`holochain_data-0.7.0/src/key.rs`), reduced to a synchronous, dependency-light
//! form.
//!
//! Why a port rather than a dependency on `holochain_data`:
//!
//! * `apply_pragmas`, `key_hex` and `salt_hex` are `pub(crate)` in
//!   `holochain_data` — they are not reachable from outside the crate at all, so
//!   the pragma strings have to be reconstructed here regardless.
//! * `DbKey::load` itself is `pub`, but reaching it drags in `holochain_types`,
//!   `holochain_conductor_api`, `sqlx` and a tokio runtime for a function whose
//!   whole body is base64-decode → argon2id → xsalsa secretbox-open.
//!
//! The on-disk format is therefore the contract, and it is fixed by
//! `holochain_data-0.7.0/src/key.rs`:
//!
//! ```text
//! db.key = base64url-nopad(
//!     nonce[24] || secretbox(key[32])[32+16] || salt[16]
//! )                                              -- 88 bytes -> 118 chars
//! secret  = argon2id(passphrase, salt, OPSLIMIT_MODERATE, MEMLIMIT_MODERATE)
//! key     = xsalsa20poly1305_open(cipher, nonce, secret)
//! ```
//!
//! `tests::round_trip_generate_then_load` pins the format from this side; opening
//! a live conductor database pins it against the conductor that wrote it.

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sodoken::{secretbox, SizedLockedArray};

/// Offsets inside the decoded `db.key` buffer.
const NONCE_LEN: usize = secretbox::XSALSA_NONCEBYTES;
const KEY_LEN: usize = 32;
const MAC_LEN: usize = secretbox::XSALSA_MACBYTES;
const SALT_LEN: usize = 16;
const LOCKED_LEN: usize = NONCE_LEN + KEY_LEN + MAC_LEN + SALT_LEN;

/// An unlocked conductor database key.
///
/// Holds the 32-byte cipher key and the 16-byte salt in `sodoken` locked
/// (`mlock`ed, `mprotect`ed) memory, exactly as `holochain_data::DbKey` does.
pub struct DbKey {
    key: SizedLockedArray<KEY_LEN>,
    salt: SizedLockedArray<SALT_LEN>,
}

impl std::fmt::Debug for DbKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbKey").finish()
    }
}

impl DbKey {
    /// Derive the argon2id secret for `passphrase` under `salt`.
    ///
    /// Mirrors the `blocking_argon2id` call in `holochain_data`'s `priv_gen` and
    /// `load`, including both limits.
    fn derive_secret(passphrase: &[u8], salt: &[u8; SALT_LEN]) -> Result<SizedLockedArray<32>> {
        let mut secret = SizedLockedArray::<32>::new().context("allocate locked argon2 output")?;
        sodoken::argon2::blocking_argon2id(
            &mut *secret.lock(),
            passphrase,
            salt,
            sodoken::argon2::ARGON2_ID_OPSLIMIT_MODERATE,
            sodoken::argon2::ARGON2_ID_MEMLIMIT_MODERATE,
        )
        .context("argon2id key derivation failed")?;
        Ok(secret)
    }

    /// Load a database key from the contents of a `db.key` file.
    ///
    /// Port of `holochain_data::DbKey::load`.
    pub fn load(locked: &str, passphrase: &[u8]) -> Result<Self> {
        let buf = URL_SAFE_NO_PAD
            .decode(locked.trim())
            .context("db.key is not base64url-nopad")?;
        if buf.len() != LOCKED_LEN {
            return Err(anyhow!(
                "db.key decodes to {} bytes, expected {} (nonce {} + key {} + mac {} + salt {})",
                buf.len(),
                LOCKED_LEN,
                NONCE_LEN,
                KEY_LEN,
                MAC_LEN,
                SALT_LEN
            ));
        }

        let mut salt = SizedLockedArray::<SALT_LEN>::new().context("allocate locked salt")?;
        salt.lock()
            .copy_from_slice(&buf[NONCE_LEN + KEY_LEN + MAC_LEN..]);

        let mut secret = {
            let salt_bytes = *salt.lock();
            Self::derive_secret(passphrase, &salt_bytes)?
        };

        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&buf[..NONCE_LEN]);
        let cipher = &buf[NONCE_LEN..NONCE_LEN + KEY_LEN + MAC_LEN];

        let mut key = SizedLockedArray::<KEY_LEN>::new().context("allocate locked db key")?;
        secretbox::xsalsa_open_easy(&mut *key.lock(), cipher, &nonce, &secret.lock()).map_err(
            |e| {
                anyhow!(
                    "could not unlock db.key with the supplied passphrase \
                     (secretbox open failed: {e})"
                )
            },
        )?;

        Ok(Self { key, salt })
    }

    /// Generate a fresh key locked by `passphrase`, returning the key and the
    /// `db.key` file contents that re-open it.
    ///
    /// Port of `holochain_data::DbKey::generate` / `priv_gen`. Only used to build
    /// encrypted fixture databases in tests — the operator verbs never mint keys.
    pub fn generate(passphrase: &[u8]) -> Result<(Self, String)> {
        let mut nonce = [0u8; NONCE_LEN];
        sodoken::random::randombytes_buf(&mut nonce).context("random nonce")?;

        let mut key = SizedLockedArray::<KEY_LEN>::new().context("allocate locked db key")?;
        sodoken::random::randombytes_buf(&mut *key.lock()).context("random db key")?;

        let mut salt = SizedLockedArray::<SALT_LEN>::new().context("allocate locked salt")?;
        sodoken::random::randombytes_buf(&mut *salt.lock()).context("random salt")?;

        let mut secret = {
            let salt_bytes = *salt.lock();
            Self::derive_secret(passphrase, &salt_bytes)?
        };

        let mut cipher = vec![0u8; KEY_LEN + MAC_LEN];
        secretbox::xsalsa_easy(&mut cipher, &nonce, &*key.lock(), &secret.lock())
            .map_err(|e| anyhow!("secretbox seal failed: {e}"))?;

        let mut buf = Vec::with_capacity(LOCKED_LEN);
        buf.extend_from_slice(&nonce);
        buf.extend_from_slice(&cipher);
        buf.extend_from_slice(&*salt.lock());

        let locked = URL_SAFE_NO_PAD.encode(&buf);
        Ok((Self { key, salt }, locked))
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02X}")).collect()
    }

    /// The SQLCipher pragma statements, in the order `holochain_data`'s
    /// `apply_pragmas` inserts them into the sqlx options map.
    ///
    /// The doubled quoting (`"x'...'"`) is deliberate and matches upstream: it is
    /// the SQLCipher raw-key form, and the outer quotes are part of the pragma
    /// value string upstream builds.
    pub fn pragma_sql(&mut self) -> String {
        let key_hex = Self::hex(&*self.key.lock());
        let salt_hex = Self::hex(&*self.salt.lock());
        format!(
            "PRAGMA key = \"x'{key_hex}'\";\n\
             PRAGMA cipher_salt = \"x'{salt_hex}'\";\n\
             PRAGMA cipher_compatibility = 4;\n\
             PRAGMA cipher_plaintext_header_size = 32;\n"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_generate_then_load() {
        let (mut minted, locked) = DbKey::generate(b"test").expect("generate");
        assert_eq!(locked.len(), 118, "db.key is 118 base64url-nopad chars");

        let mut reloaded = DbKey::load(&locked, b"test").expect("load");
        assert_eq!(minted.pragma_sql(), reloaded.pragma_sql());
    }

    #[test]
    fn wrong_passphrase_is_refused() {
        let (_, locked) = DbKey::generate(b"test").expect("generate");
        let err = DbKey::load(&locked, b"not-the-passphrase").expect_err("must refuse");
        assert!(
            err.to_string().contains("could not unlock db.key"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn malformed_key_file_is_refused() {
        let err = DbKey::load("AAAA", b"test").expect_err("must refuse");
        assert!(err.to_string().contains("expected 88"), "got: {err}");
    }
}
