//! Custodial Key Management for Hosted Humans
//!
//! Provides secure key generation, storage, and signing for users
//! who don't yet have their own Holochain node (Tauri app).
//!
//! # Architecture
//!
//! Each hosted human gets their own Ed25519 keypair at registration:
//! - Public key becomes their Holochain agent identity
//! - Private key is encrypted with their password (Argon2id + ChaCha20-Poly1305)
//! - Keys are stored in MongoDB, cached in memory during active sessions
//!
//! # Migration to Stewardship
//!
//! When users are ready to run their own node (Tauri app):
//! 1. Export encrypted key bundle via `/auth/export-key`
//! 2. Import to Tauri, decrypt with password
//! 3. Confirm stewardship via `/auth/confirm-stewardship`
//! 4. Doorway retires conductor cell, user is now a steward

pub mod cache;
pub mod crypto;
pub mod service;

pub use cache::{CachedSigningKey, SigningKeyCache, SigningKeyCacheConfig};
pub use crypto::{
    decrypt_private_key, derive_key_encryption_key, encrypt_private_key, generate_keypair,
    generate_random_bytes, sign_payload, NONCE_LEN, SALT_LEN,
};
pub use service::{CustodialKeyService, KeyExportFormat};
