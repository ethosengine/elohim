//! Wire-format `Observation` struct — MessagePack/CBOR over the
//! `observation-log` ALPN.
//!
//! Conventions:
//! - `#[serde(rename_all = "camelCase")]` per the protocol's Rust-to-TS
//!   boundary rule (CLAUDE.md).
//! - `payload_json` is pre-stringified to avoid `serde_json::Value` on the
//!   wire — see `feedback_serde_json_value_breaks_zome_boundary` memory pin.
//! - `signature` covers all fields above it via `canonical_signing_bytes`
//!   (added in Task 2.2; not in scope here).

use serde::{Deserialize, Serialize};

/// A single observation — one row in an observer's append-only iroh-blob log.
///
/// Universal reference: `(observer_cid, log_cid, log_offset)`. Attestation
/// `metadata.observation_refs` serialises this triple as
/// `iroh://<observer_cid>@<log_cid>#<offset>`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub observer_cid: String,
    pub log_cid: String,
    pub log_offset: u64,
    pub observed_at: i64,
    pub seq: u64,

    pub observation_kind: String,
    pub subject_cid: Option<String>,
    pub subject_kind: Option<String>,
    pub payload_json: String,

    pub observer_household_cid: Option<String>,
    pub observer_collective_cid: Option<String>,
    pub observer_region: Option<String>,
    pub observer_archetype: Option<String>,
    pub observer_compute_class: Option<String>,

    pub signature: Vec<u8>,
}
