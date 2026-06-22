//! Private-replica encryption — Wave 5.1 Slice-0 PROOF.
//!
//! Single-host, DB-free, network-free cryptographic proof of the
//! **encrypt-then-erasure-code** round-trip for a private blob, plus the
//! **key-envelope** mechanic that will later back a `KeyEnvelope` DHT entry.
//!
//! ## Why this exists
//!
//! Private content must be encrypted *before* it is erasure-coded and handed to
//! custodian peers — a custodian holding a shard must never be able to read the
//! plaintext. This module proves the full pipeline end-to-end on a single host:
//!
//! ```text
//! plaintext
//!   → DEK = crypto_secretbox_keygen()            (random 32-byte data key)
//!   → ciphertext = secretbox(plaintext, DEK, nonce)   (XSalsa20-Poly1305)
//!   → shards = Reed-Solomon erasure-code(ciphertext)  (rs-4-7)
//!   → DROP one shard                              (simulate a lost custodian)
//!   → ciphertext' = reconstruct(surviving shards) (RS recovery)
//!   → plaintext'  = secretbox_open(ciphertext', DEK, nonce)
//!   → assert plaintext' == plaintext  &&  cid(plaintext') == cid(plaintext)
//! ```
//!
//! Separately it proves the **key envelope**: the DEK is sealed to a reader's
//! X25519 public key via `crypto_box_seal` and unsealed with the reader's secret
//! key — the cheap stand-in for the future `KeyEnvelope` DHT entry. Each reader
//! gets their own sealed copy of the DEK; the ciphertext + shards are shared.
//!
//! ## Scope (held / out of scope)
//!
//! This is a PROOF, not the production surface. It does NOT build the real
//! `KeyEnvelope` DHT entry, a `ShardManifest` field-add, or any X25519
//! reader-key resolver — those are HELD. The public surface here is the minimum
//! a future caller needs to wire the pipeline together.
//!
//! ## Nonce handling
//!
//! secretbox requires the 24-byte nonce at decrypt time. The nonce is public
//! (it provides uniqueness, not secrecy), so we prepend it to the ciphertext
//! before sharding and split it back off after reconstruction. This keeps the
//! "ciphertext blob" self-describing: everything needed to decrypt (given the
//! DEK) travels with the erasure-coded payload.

use crate::blob_store::BlobStore;
use crate::sharding::{ShardConfig, ShardEncoder, ShardManifest};
use dryoc::classic::crypto_box::{crypto_box_seal, crypto_box_seal_open, PublicKey, SecretKey};
use dryoc::classic::crypto_secretbox::{
    crypto_secretbox_easy, crypto_secretbox_keygen, crypto_secretbox_open_easy, Key, Nonce,
};
use dryoc::constants::{
    CRYPTO_BOX_SEALBYTES, CRYPTO_SECRETBOX_MACBYTES, CRYPTO_SECRETBOX_NONCEBYTES,
};
use dryoc::types::NewByteArray;

/// Errors produced by the private-replica proof pipeline.
#[derive(Debug, thiserror::Error)]
pub enum PrivateReplicaError {
    /// Symmetric encryption / decryption failed (wrong DEK or tampered bytes).
    #[error("symmetric crypto failure: {0}")]
    Symmetric(String),
    /// Key-envelope seal / unseal failed (wrong reader key or tampered envelope).
    #[error("key-envelope failure: {0}")]
    Envelope(String),
    /// Reed-Solomon shard encoding or reconstruction failed.
    #[error("erasure-coding failure: {0}")]
    Erasure(String),
    /// A ciphertext blob was shorter than the prepended nonce — malformed input.
    #[error("ciphertext blob too short to contain a nonce")]
    Malformed,
}

/// The 32-byte data encryption key. Reuses dryoc's secretbox key type so the
/// same bytes serve both as the symmetric key and as the payload sealed into
/// the key envelope.
pub type Dek = Key;

/// Output of [`encrypt_then_shard`]: everything a custodian set holds for a
/// private blob.
///
/// The shards together with [`manifest`](Self::manifest) are what get
/// distributed to peers; none of them reveal the plaintext without the DEK.
pub struct EncryptedReplica {
    /// Reed-Solomon shards of the (nonce ++ ciphertext) blob. `data_shards` of
    /// these (any subset of that size) suffice to reconstruct.
    pub shards: Vec<Vec<u8>>,
    /// Manifest describing the erasure coding — drives reconstruction. Its
    /// `total_size` is the length of the (nonce ++ ciphertext) blob, NOT the
    /// plaintext, so reconstruction restores the exact blob.
    pub manifest: ShardManifest,
}

/// Build a [`ShardEncoder`] configured to ALWAYS use Reed-Solomon (rs-4-7),
/// regardless of how small the ciphertext is. The default encoder would pick
/// the "none" band for tiny payloads, which has no parity and cannot survive a
/// dropped shard — defeating the whole proof.
fn rs_encoder() -> ShardEncoder {
    ShardEncoder::new(ShardConfig {
        // Force the RS band for any input ≥ 1 byte: never "none", never "chunked".
        single_shard_max: 0,
        rs_threshold: 0,
        ..ShardConfig::default()
    })
}

/// Symmetrically encrypt `plaintext` under a freshly generated DEK, then
/// erasure-code the ciphertext into shards.
///
/// Returns the generated DEK alongside the [`EncryptedReplica`]. The DEK is the
/// secret that must be sealed (see [`seal_dek`]) to each authorized reader; the
/// replica is the shared, custodian-distributable payload.
///
/// The on-disk/wire ciphertext blob is `nonce (24B) ++ secretbox_ciphertext`,
/// so it carries everything needed to decrypt given the DEK.
pub fn encrypt_then_shard(
    plaintext: &[u8],
) -> Result<(Dek, EncryptedReplica), PrivateReplicaError> {
    // 1. Generate a random 32-byte DEK (the data encryption key).
    let dek: Dek = crypto_secretbox_keygen();

    // 2. Generate a random 24-byte nonce and encrypt under the DEK.
    //    `crypto_secretbox_easy` writes into a caller-sized buffer:
    //    MAC (16B) prepended to the ciphertext, total = MACBYTES + plaintext.len().
    let nonce: Nonce = Nonce::gen();
    let mut ciphertext = vec![0u8; CRYPTO_SECRETBOX_MACBYTES + plaintext.len()];
    crypto_secretbox_easy(&mut ciphertext, plaintext, &nonce, &dek)
        .map_err(|e| PrivateReplicaError::Symmetric(e.to_string()))?;

    // 3. Frame as a self-describing blob: nonce ++ ciphertext. The nonce is
    //    public; sharding it alongside the ciphertext is safe.
    let mut blob = Vec::with_capacity(CRYPTO_SECRETBOX_NONCEBYTES + ciphertext.len());
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);

    // 4. Reed-Solomon erasure-code the ciphertext blob.
    let encoder = rs_encoder();
    let manifest = encoder
        .create_manifest(&blob, "application/octet-stream", "private")
        .map_err(|e| PrivateReplicaError::Erasure(e.to_string()))?;
    let shards = encoder
        .create_shards(&blob, &manifest.encoding)
        .map_err(|e| PrivateReplicaError::Erasure(e.to_string()))?;

    Ok((dek, EncryptedReplica { shards, manifest }))
}

/// Reconstruct the ciphertext blob from a (possibly incomplete) set of shards,
/// then symmetrically decrypt it under `dek` to recover the plaintext.
///
/// `shards` is positional and parallel to the manifest's shard order: a `None`
/// entry marks a missing shard (e.g. a lost custodian). Reconstruction succeeds
/// as long as at least `manifest.data_shards` shards are present.
pub fn reconstruct_then_decrypt(
    dek: &Dek,
    manifest: &ShardManifest,
    shards: &[Option<Vec<u8>>],
) -> Result<Vec<u8>, PrivateReplicaError> {
    // 1. Reed-Solomon reconstruct the (nonce ++ ciphertext) blob.
    let encoder = rs_encoder();
    let blob = encoder
        .reconstruct(manifest, shards)
        .map_err(|e| PrivateReplicaError::Erasure(e.to_string()))?;

    // 2. Split the public nonce back off the front.
    if blob.len() < CRYPTO_SECRETBOX_NONCEBYTES + CRYPTO_SECRETBOX_MACBYTES {
        return Err(PrivateReplicaError::Malformed);
    }
    let (nonce_bytes, ciphertext) = blob.split_at(CRYPTO_SECRETBOX_NONCEBYTES);
    let mut nonce: Nonce = [0u8; CRYPTO_SECRETBOX_NONCEBYTES];
    nonce.copy_from_slice(nonce_bytes);

    // 3. Symmetrically decrypt under the DEK. A wrong DEK fails authentication.
    //    `crypto_secretbox_open_easy` writes the recovered plaintext into a
    //    caller-sized buffer: ciphertext.len() - MACBYTES.
    let mut plaintext = vec![0u8; ciphertext.len() - CRYPTO_SECRETBOX_MACBYTES];
    crypto_secretbox_open_easy(&mut plaintext, ciphertext, &nonce, dek)
        .map_err(|e| PrivateReplicaError::Symmetric(e.to_string()))?;
    Ok(plaintext)
}

/// Seal the DEK to a reader's X25519 public key — the cheap stand-in for a
/// future `KeyEnvelope` DHT entry.
///
/// Uses `crypto_box_seal` (anonymous sealed-box: ephemeral sender keypair,
/// X25519/XSalsa20/Poly1305), mirroring [`crate::services::sealed_against_self`].
/// Only the holder of the matching secret key can [`unseal_dek`] it.
pub fn seal_dek(dek: &Dek, reader_pk: &PublicKey) -> Result<Vec<u8>, PrivateReplicaError> {
    let mut envelope = vec![0u8; dek.len() + CRYPTO_BOX_SEALBYTES];
    crypto_box_seal(&mut envelope, dek, reader_pk)
        .map_err(|e| PrivateReplicaError::Envelope(e.to_string()))?;
    Ok(envelope)
}

/// Unseal a DEK envelope with the reader's keypair, recovering the 32-byte DEK.
///
/// Returns `Err` if the envelope was sealed to a different reader or has been
/// tampered with (sealed-box authentication failure).
pub fn unseal_dek(
    envelope: &[u8],
    reader_pk: &PublicKey,
    reader_sk: &SecretKey,
) -> Result<Dek, PrivateReplicaError> {
    if envelope.len() < CRYPTO_BOX_SEALBYTES {
        return Err(PrivateReplicaError::Envelope(format!(
            "envelope too short: {} bytes",
            envelope.len()
        )));
    }
    let dek_len = envelope.len() - CRYPTO_BOX_SEALBYTES;
    let mut dek_bytes = vec![0u8; dek_len];
    crypto_box_seal_open(&mut dek_bytes, envelope, reader_pk, reader_sk)
        .map_err(|e| PrivateReplicaError::Envelope(e.to_string()))?;

    let mut dek: Dek = [0u8; std::mem::size_of::<Dek>()];
    if dek_bytes.len() != dek.len() {
        return Err(PrivateReplicaError::Envelope(format!(
            "recovered key has wrong length: {} bytes",
            dek_bytes.len()
        )));
    }
    dek.copy_from_slice(&dek_bytes);
    Ok(dek)
}

/// Canonical content address of `bytes` — the crate's real content-address, so
/// identity of recovered plaintext is verified the same way the rest of the
/// substrate addresses content. A future caller verifies a recovered private
/// blob by comparing `plaintext_cid(recovered) == plaintext_cid(original)`.
pub fn plaintext_cid(bytes: &[u8]) -> String {
    BlobStore::compute_cid(bytes).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dryoc::classic::crypto_box::crypto_box_seed_keypair;

    /// Deterministic reader keypair from a seed (test stability only — prod
    /// readers carry real X25519 identity keys).
    fn reader_keypair(seed: u8) -> (PublicKey, SecretKey) {
        crypto_box_seed_keypair(&[seed; 32])
    }

    /// Drop exactly one shard (the `idx`th) to simulate a lost custodian,
    /// returning the positional `Option` array reconstruction expects.
    fn drop_one(shards: &[Vec<u8>], idx: usize) -> Vec<Option<Vec<u8>>> {
        shards
            .iter()
            .enumerate()
            .map(|(i, s)| if i == idx { None } else { Some(s.clone()) })
            .collect()
    }

    // -------------------------------------------------------------------------
    // Test 1: full round-trip with a dropped shard recovers byte-identical
    //         plaintext AND an equal plaintext CID.
    // -------------------------------------------------------------------------
    #[test]
    fn round_trip_survives_dropped_shard() {
        let plaintext = b"private content the custodian must never read in the clear";

        let (dek, replica) = encrypt_then_shard(plaintext).expect("encrypt_then_shard");

        // The replica is rs-4-7: 4 data + 3 parity = 7 shards.
        assert_eq!(replica.manifest.encoding, "rs-4-7");
        assert_eq!(replica.shards.len(), 7);

        // Drop one shard (a parity shard here) — a lost custodian.
        let with_loss = drop_one(&replica.shards, 6);

        let recovered =
            reconstruct_then_decrypt(&dek, &replica.manifest, &with_loss).expect("reconstruct");

        assert_eq!(
            recovered.as_slice(),
            plaintext.as_slice(),
            "recovered plaintext must be byte-identical"
        );
        assert_eq!(
            plaintext_cid(&recovered),
            plaintext_cid(plaintext),
            "recovered plaintext CID must equal original"
        );
    }

    /// Reconstruction tolerates dropping ANY single shard (data or parity),
    /// since rs-4-7 survives up to 3 losses.
    #[test]
    fn round_trip_survives_dropped_data_shard() {
        let plaintext = b"another private payload, this time we drop a data shard";
        let (dek, replica) = encrypt_then_shard(plaintext).expect("encrypt_then_shard");

        let with_loss = drop_one(&replica.shards, 0); // drop a DATA shard
        let recovered =
            reconstruct_then_decrypt(&dek, &replica.manifest, &with_loss).expect("reconstruct");

        assert_eq!(recovered.as_slice(), plaintext.as_slice());
    }

    // -------------------------------------------------------------------------
    // Test 2: the DEK envelope seals and unseals back to the SAME DEK.
    // -------------------------------------------------------------------------
    #[test]
    fn dek_envelope_round_trips() {
        let (dek, _replica) = encrypt_then_shard(b"x").expect("encrypt_then_shard");
        let (reader_pk, reader_sk) = reader_keypair(7);

        let envelope = seal_dek(&dek, &reader_pk).expect("seal_dek");
        // The envelope must be opaque — it is not the bare DEK.
        assert_ne!(envelope.as_slice(), dek.as_slice());

        let recovered_dek = unseal_dek(&envelope, &reader_pk, &reader_sk).expect("unseal_dek");
        assert_eq!(
            recovered_dek, dek,
            "DEK must round-trip through the envelope"
        );
    }

    /// End-to-end: a reader who receives ONLY (shards, manifest, sealed
    /// envelope) — never the bare DEK — can recover the plaintext, while a
    /// custodian holding shards alone cannot.
    #[test]
    fn reader_with_envelope_recovers_custodian_cannot() {
        let plaintext = b"the knowledge belongs to the reader, not the custodian";
        let (dek, replica) = encrypt_then_shard(plaintext).expect("encrypt_then_shard");

        let (reader_pk, reader_sk) = reader_keypair(11);
        let envelope = seal_dek(&dek, &reader_pk).expect("seal_dek");

        // Reader path: unseal the DEK, then reconstruct + decrypt (one shard lost).
        let readers_dek = unseal_dek(&envelope, &reader_pk, &reader_sk).expect("unseal_dek");
        let with_loss = drop_one(&replica.shards, 5);
        let recovered = reconstruct_then_decrypt(&readers_dek, &replica.manifest, &with_loss)
            .expect("reader recovers");
        assert_eq!(recovered.as_slice(), plaintext.as_slice());

        // Custodian path: holds all shards, reconstructs the ciphertext blob,
        // but WITHOUT the DEK the bytes are opaque — and contain no plaintext.
        let all_present: Vec<Option<Vec<u8>>> =
            replica.shards.iter().map(|s| Some(s.clone())).collect();
        let encoder = rs_encoder();
        let ct_blob = encoder
            .reconstruct(&replica.manifest, &all_present)
            .expect("custodian reconstructs ciphertext blob");
        // Ciphertext blob (nonce ++ ct) must NOT contain the plaintext.
        assert_ne!(ct_blob.as_slice(), plaintext.as_slice());
        assert!(
            !contains_subsequence(&ct_blob, plaintext),
            "encrypted blob must not leak plaintext to a custodian"
        );
    }

    // -------------------------------------------------------------------------
    // Test 3 (negative): encryption is load-bearing.
    //   (a) wrong DEK fails to decrypt;
    //   (b) too many dropped shards fails to reconstruct;
    //   (c) the envelope opened by the wrong reader fails.
    // -------------------------------------------------------------------------
    #[test]
    fn wrong_dek_fails_to_decrypt() {
        let plaintext = b"load-bearing encryption: a wrong key must not decrypt";
        let (_dek, replica) = encrypt_then_shard(plaintext).expect("encrypt_then_shard");

        // A different, valid 32-byte key — correct nonce travels in the blob.
        let wrong_dek: Dek = crypto_secretbox_keygen();
        let all_present: Vec<Option<Vec<u8>>> =
            replica.shards.iter().map(|s| Some(s.clone())).collect();

        let result = reconstruct_then_decrypt(&wrong_dek, &replica.manifest, &all_present);
        assert!(
            matches!(result, Err(PrivateReplicaError::Symmetric(_))),
            "wrong DEK must fail authentication, got {result:?}"
        );
    }

    #[test]
    fn too_many_dropped_shards_fails_to_reconstruct() {
        let plaintext = b"rs-4-7 cannot survive losing four shards";
        let (dek, replica) = encrypt_then_shard(plaintext).expect("encrypt_then_shard");

        // rs-4-7 needs 4 of 7; drop 4 → only 3 present → reconstruction fails.
        let mut with_loss: Vec<Option<Vec<u8>>> =
            replica.shards.iter().map(|s| Some(s.clone())).collect();
        for slot in with_loss.iter_mut().take(4) {
            *slot = None;
        }

        let result = reconstruct_then_decrypt(&dek, &replica.manifest, &with_loss);
        assert!(
            matches!(result, Err(PrivateReplicaError::Erasure(_))),
            "losing 4 of 7 shards must fail reconstruction, got {result:?}"
        );
    }

    #[test]
    fn wrong_reader_cannot_unseal_envelope() {
        let (dek, _replica) = encrypt_then_shard(b"y").expect("encrypt_then_shard");
        let (reader_pk, _reader_sk) = reader_keypair(1);
        let (_other_pk, other_sk) = reader_keypair(2);

        let envelope = seal_dek(&dek, &reader_pk).expect("seal_dek");

        // The intended reader's PUBLIC key with the WRONG secret key must fail.
        let result = unseal_dek(&envelope, &reader_pk, &other_sk);
        assert!(
            matches!(result, Err(PrivateReplicaError::Envelope(_))),
            "wrong reader secret key must fail to unseal, got {result:?}"
        );
    }

    /// Naive subsequence check used to assert the ciphertext does not contain
    /// the plaintext (encryption is real, not a no-op).
    fn contains_subsequence(haystack: &[u8], needle: &[u8]) -> bool {
        if needle.is_empty() || needle.len() > haystack.len() {
            return false;
        }
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }
}
