//! Sealed-against-self — interim 2-of-2 encryption per brainstorm §10.1.
//!
//! Phase 3.5 ships a 2-of-2 (mishpat-quorum + subject's imagodei) sealed-box
//! using dryoc's `crypto_box_seal` (X25519/XSalsa20/Poly1305 sealed-box).
//! Phase 5/6 will replace with a t-of-n threshold scheme; the property
//! (recovery requires governance cooperation, never unilateral disclosure) is
//! canonical here.
//!
//! ## Nesting contract
//!
//! ```text
//! seal:
//!   inner_ct = crypto_box_seal(plaintext,    imagodei_pk)
//!   outer_ct = crypto_box_seal(inner_ct,     mishpat_pk)
//!   SealedBlob { ciphertext: outer_ct }
//!
//! unseal:
//!   inner_ct = crypto_box_seal_open(outer_ct, mishpat_pk, mishpat_sk)
//!   plaintext = crypto_box_seal_open(inner_ct, imagodei_pk, imagodei_sk)
//! ```
//!
//! Either key alone reveals only ciphertext bytes, never the plaintext.
//!
//! ## Plan deviation note
//!
//! The brainstorm plan proposed `SealedBlob { mishpat_outer, imagodei_inner }`
//! (two separate fields). That shape is NOT 2-of-2: each key could independently
//! decrypt half the plaintext. This implementation uses a single `ciphertext`
//! field (true nesting) which is the only shape that satisfies the
//! "either key alone yields nothing useful" requirement.

use dryoc::classic::crypto_box::{crypto_box_seal, crypto_box_seal_open, PublicKey, SecretKey};
use dryoc::constants::CRYPTO_BOX_SEALBYTES;
use serde::{Deserialize, Serialize};

/// A 2-of-2 sealed blob.
///
/// `ciphertext` is `crypto_box_seal(crypto_box_seal(plaintext, imagodei_pk), mishpat_pk)`.
/// Mishpat-quorum unseals first, revealing the inner ciphertext; imagodei then
/// unseals to the original plaintext. Either key alone reveals only opaque bytes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealedBlob {
    /// Outer ciphertext: nested sealed boxes.
    /// Unseal order: mishpat_sk → inner ciphertext → imagodei_sk → plaintext.
    pub ciphertext: Vec<u8>,
}

/// Errors produced by seal / unseal operations.
#[derive(Debug, thiserror::Error)]
pub enum SealError {
    /// The underlying crypto primitive failed (malformed input, wrong key, or
    /// tampered ciphertext).
    #[error("crypto failure: {0}")]
    Crypto(String),
    /// The outer layer decrypted successfully but the inner ciphertext is too
    /// short to be a valid sealed box — indicates a logic error or tamper.
    #[error("decryption requires both keys; got partial decrypt")]
    PartialDecrypt,
}

/// Seal `plaintext` into a 2-of-2 nested box.
///
/// Nesting order (innermost first):
/// 1. `crypto_box_seal(plaintext, imagodei_pk)` → inner ciphertext
/// 2. `crypto_box_seal(inner_ct,  mishpat_pk)`  → outer ciphertext stored in blob
///
/// Both keys are required to recover `plaintext`.
pub fn seal(
    plaintext: &[u8],
    mishpat_pk: &PublicKey,
    imagodei_pk: &PublicKey,
) -> Result<SealedBlob, SealError> {
    // Step 1: seal plaintext to imagodei (inner layer)
    let inner_len = plaintext.len() + CRYPTO_BOX_SEALBYTES;
    let mut inner_ct = vec![0u8; inner_len];
    crypto_box_seal(&mut inner_ct, plaintext, imagodei_pk)
        .map_err(|e| SealError::Crypto(e.to_string()))?;

    // Step 2: seal inner ciphertext to mishpat-quorum (outer layer)
    let outer_len = inner_ct.len() + CRYPTO_BOX_SEALBYTES;
    let mut outer_ct = vec![0u8; outer_len];
    crypto_box_seal(&mut outer_ct, &inner_ct, mishpat_pk)
        .map_err(|e| SealError::Crypto(e.to_string()))?;

    Ok(SealedBlob {
        ciphertext: outer_ct,
    })
}

/// Unseal a 2-of-2 nested blob.
///
/// Unseal order (outermost first):
/// 1. `crypto_box_seal_open(outer_ct, mishpat_pk, mishpat_sk)` → inner ciphertext
/// 2. `crypto_box_seal_open(inner_ct, imagodei_pk, imagodei_sk)` → plaintext
///
/// Returns `Err(SealError::Crypto)` if either layer fails authentication —
/// which happens when any key is wrong OR the ciphertext has been tampered with.
pub fn unseal(
    sealed: &SealedBlob,
    mishpat_pk: &PublicKey,
    mishpat_sk: &SecretKey,
    imagodei_pk: &PublicKey,
    imagodei_sk: &SecretKey,
) -> Result<Vec<u8>, SealError> {
    let outer_ct = &sealed.ciphertext;

    // Step 1: outer decrypt — requires mishpat keys
    if outer_ct.len() < CRYPTO_BOX_SEALBYTES {
        return Err(SealError::Crypto(format!(
            "outer ciphertext too short: {} bytes",
            outer_ct.len()
        )));
    }
    let inner_len = outer_ct.len() - CRYPTO_BOX_SEALBYTES;
    let mut inner_ct = vec![0u8; inner_len];
    crypto_box_seal_open(&mut inner_ct, outer_ct, mishpat_pk, mishpat_sk)
        .map_err(|e| SealError::Crypto(format!("outer unseal failed: {e}")))?;

    // Step 2: inner decrypt — requires imagodei keys
    if inner_ct.len() < CRYPTO_BOX_SEALBYTES {
        // The outer layer decrypted but the inner bytes aren't a valid sealed box.
        // This should not occur with well-formed blobs; treat as tamper evidence.
        return Err(SealError::PartialDecrypt);
    }
    let plaintext_len = inner_ct.len() - CRYPTO_BOX_SEALBYTES;
    let mut plaintext = vec![0u8; plaintext_len];
    crypto_box_seal_open(&mut plaintext, &inner_ct, imagodei_pk, imagodei_sk)
        .map_err(|e| SealError::Crypto(format!("inner unseal failed: {e}")))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dryoc::classic::crypto_box::crypto_box_seed_keypair;

    // Golden vector test deferred — crypto_box_seal uses ephemeral randomness;
    // would require deterministic RNG injection. Round-trip with deterministic
    // keypairs is sufficient stability proof for Phase 3.5.

    /// Derive a deterministic keypair from a 32-byte seed.
    fn keypair_from_seed(seed: [u8; 32]) -> (PublicKey, SecretKey) {
        crypto_box_seed_keypair(&seed)
    }

    fn mishpat_keypair() -> (PublicKey, SecretKey) {
        keypair_from_seed([0u8; 32])
    }

    fn imagodei_keypair() -> (PublicKey, SecretKey) {
        keypair_from_seed([1u8; 32])
    }

    // -------------------------------------------------------------------------
    // Test 1: Round-trip with both keys succeeds
    // -------------------------------------------------------------------------
    #[test]
    fn round_trip_both_keys() {
        let (mishpat_pk, mishpat_sk) = mishpat_keypair();
        let (imagodei_pk, imagodei_sk) = imagodei_keypair();

        let plaintext = b"the knowledge belongs to the community";

        let sealed = seal(plaintext, &mishpat_pk, &imagodei_pk).expect("seal failed");
        let recovered = unseal(
            &sealed,
            &mishpat_pk,
            &mishpat_sk,
            &imagodei_pk,
            &imagodei_sk,
        )
        .expect("unseal failed");

        assert_eq!(recovered.as_slice(), plaintext);
    }

    // -------------------------------------------------------------------------
    // Test 2: mishpat_sk only — outer decrypts to inner ciphertext, NOT plaintext
    //
    // We open the outer layer manually and assert the bytes do NOT equal the
    // plaintext. (We cannot call the full `unseal` without imagodei keys.)
    // -------------------------------------------------------------------------
    #[test]
    fn mishpat_only_reveals_inner_ciphertext_not_plaintext() {
        let (mishpat_pk, mishpat_sk) = mishpat_keypair();
        let (imagodei_pk, _imagodei_sk) = imagodei_keypair();

        let plaintext = b"the knowledge belongs to the community";

        let sealed = seal(plaintext, &mishpat_pk, &imagodei_pk).expect("seal failed");

        // Manually open the outer layer — simulates mishpat-quorum acting alone.
        let outer_ct = &sealed.ciphertext;
        let inner_len = outer_ct.len() - CRYPTO_BOX_SEALBYTES;
        let mut inner_ct = vec![0u8; inner_len];
        crypto_box_seal_open(&mut inner_ct, outer_ct, &mishpat_pk, &mishpat_sk)
            .expect("outer open should succeed with mishpat keys");

        // The result is the inner ciphertext — NOT the plaintext.
        assert_ne!(
            inner_ct.as_slice(),
            plaintext.as_slice(),
            "mishpat alone must not reveal plaintext"
        );
    }

    // -------------------------------------------------------------------------
    // Test 3: imagodei_sk only — outer fails (wrong keys for outer layer)
    // -------------------------------------------------------------------------
    #[test]
    fn imagodei_only_fails_on_outer() {
        let (mishpat_pk, _mishpat_sk) = mishpat_keypair();
        let (imagodei_pk, imagodei_sk) = imagodei_keypair();

        let plaintext = b"the knowledge belongs to the community";

        let sealed = seal(plaintext, &mishpat_pk, &imagodei_pk).expect("seal failed");

        // Attempt outer open with imagodei keys (wrong key for outer layer) —
        // must fail.
        let outer_ct = &sealed.ciphertext;
        let inner_len = outer_ct.len().saturating_sub(CRYPTO_BOX_SEALBYTES);
        let mut buf = vec![0u8; inner_len];
        let result = crypto_box_seal_open(&mut buf, outer_ct, &imagodei_pk, &imagodei_sk);

        assert!(
            result.is_err(),
            "imagodei alone must not open the outer layer"
        );
    }

    // -------------------------------------------------------------------------
    // Test 4: Tampered outer ciphertext — unseal fails
    // -------------------------------------------------------------------------
    #[test]
    fn tampered_outer_fails() {
        let (mishpat_pk, mishpat_sk) = mishpat_keypair();
        let (imagodei_pk, imagodei_sk) = imagodei_keypair();

        let plaintext = b"the knowledge belongs to the community";

        let mut sealed = seal(plaintext, &mishpat_pk, &imagodei_pk).expect("seal failed");

        // Flip a byte in the middle of the outer ciphertext.
        let mid = sealed.ciphertext.len() / 2;
        sealed.ciphertext[mid] ^= 0xFF;

        let result = unseal(
            &sealed,
            &mishpat_pk,
            &mishpat_sk,
            &imagodei_pk,
            &imagodei_sk,
        );
        assert!(result.is_err(), "tampered outer must fail");
    }

    // -------------------------------------------------------------------------
    // Test 5: Tampered inner ciphertext — fails after outer decrypt
    //
    // We seal a blob, then manually tamper the inner ciphertext (by decrypting
    // the outer, flipping a byte, re-encrypting to mishpat isn't straightforward
    // without raw access, so instead we construct a `SealedBlob` with a
    // deliberately invalid inner payload wrapped in a fresh outer seal).
    // -------------------------------------------------------------------------
    #[test]
    fn tampered_inner_fails() {
        let (mishpat_pk, mishpat_sk) = mishpat_keypair();
        let (imagodei_pk, imagodei_sk) = imagodei_keypair();

        let plaintext = b"the knowledge belongs to the community";

        // Produce a valid inner ciphertext, then tamper it, then re-wrap in outer.
        let inner_len = plaintext.len() + CRYPTO_BOX_SEALBYTES;
        let mut inner_ct = vec![0u8; inner_len];
        crypto_box_seal(&mut inner_ct, plaintext, &imagodei_pk).expect("inner seal failed");

        // Flip a byte in the inner ciphertext.
        let mid = inner_ct.len() / 2;
        inner_ct[mid] ^= 0xFF;

        // Re-wrap the tampered inner ct in the outer seal.
        let outer_len = inner_ct.len() + CRYPTO_BOX_SEALBYTES;
        let mut outer_ct = vec![0u8; outer_len];
        crypto_box_seal(&mut outer_ct, &inner_ct, &mishpat_pk).expect("outer seal failed");

        let tampered_blob = SealedBlob {
            ciphertext: outer_ct,
        };

        let result = unseal(
            &tampered_blob,
            &mishpat_pk,
            &mishpat_sk,
            &imagodei_pk,
            &imagodei_sk,
        );
        assert!(
            result.is_err(),
            "tampered inner must fail on imagodei unseal"
        );
    }

    // -------------------------------------------------------------------------
    // Test 6: Empty plaintext round-trips correctly
    // -------------------------------------------------------------------------
    #[test]
    fn empty_plaintext_round_trips() {
        let (mishpat_pk, mishpat_sk) = mishpat_keypair();
        let (imagodei_pk, imagodei_sk) = imagodei_keypair();

        let plaintext: &[u8] = b"";

        let sealed = seal(plaintext, &mishpat_pk, &imagodei_pk).expect("seal failed");
        let recovered = unseal(
            &sealed,
            &mishpat_pk,
            &mishpat_sk,
            &imagodei_pk,
            &imagodei_sk,
        )
        .expect("unseal failed");

        assert_eq!(recovered.as_slice(), plaintext);
    }
}
