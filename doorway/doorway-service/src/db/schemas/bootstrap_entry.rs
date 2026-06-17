//! Kitsune2 bootstrap entry schema (shared bootstrap table)
//!
//! One stored kitsune2 agent-info record. Lives in a FIXED shared database
//! (`elohim-bootstrap`, independent of the per-doorway projection DB) so every
//! doorway replica/edge reads and writes ONE bootstrap table — the structural
//! fix for genesis-pair islanding (each doorway previously held a disjoint
//! per-pod in-memory store, so the two genesis peers could never discover each
//! other).
//!
//! - The mongo TTL index on `expires_at` replaces the per-pod 60s cleanup task
//!   and survives pod restart, killing "fresh per boot."
//! - The unique compound `(space, agent)` index makes the per-space eviction
//!   deterministic across replicas (one row per agent, refreshed on the
//!   client's heartbeat cadence) instead of the per-pod arbitrary eviction.
//!
//! Cat-C operational: edge-local ephemeral discovery cache, TTL-bounded. The
//! agent-info body is itself ed25519-signed (validated at PUT in `bootstrap::k2`),
//! so a row is a transient mirror of a signed external artifact, not a source
//! of truth.

use bson::{doc, oid::ObjectId, DateTime, Document};
use mongodb::options::IndexOptions;
use serde::{Deserialize, Serialize};

use super::metadata::Metadata;
use crate::db::mongo::IntoIndexes;

/// Collection name for kitsune2 bootstrap entries (in the `elohim-bootstrap` DB).
pub const BOOTSTRAP_COLLECTION: &str = "k2_bootstrap";

/// One stored kitsune2 agent-info record.
///
/// `Default` is implemented manually (not derived) because `bson::DateTime`
/// has no `Default`; `expires_at` defaults to "now".
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BootstrapEntryDoc {
    /// MongoDB document ID
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    /// Standard metadata (created_at, updated_at, is_deleted)
    #[serde(default)]
    pub metadata: Metadata,

    /// b64url space id (the kitsune space — DNA hash projection)
    #[serde(default)]
    pub space: String,

    /// b64url agent id
    #[serde(default)]
    pub agent: String,

    /// Verbatim signed agent-info PUT body (stored as BSON Binary so the GET
    /// array returns byte-identical bytes to what `decode_list` expects).
    #[serde(with = "serde_bytes", default)]
    pub raw_body: Vec<u8>,

    /// TTL key — wall-clock expiry from the signed body's `expiresAt`. The
    /// mongo TTL index fires when this is reached.
    #[serde(default = "DateTime::now")]
    pub expires_at: DateTime,
}

impl Default for BootstrapEntryDoc {
    fn default() -> Self {
        Self {
            id: None,
            metadata: Metadata::default(),
            space: String::new(),
            agent: String::new(),
            raw_body: Vec::new(),
            expires_at: DateTime::now(),
        }
    }
}

impl IntoIndexes for BootstrapEntryDoc {
    fn into_indices() -> Vec<(Document, Option<IndexOptions>)> {
        vec![
            // TTL index — entries are dropped automatically when `expires_at`
            // reaches wall-clock, replacing the per-pod cleanup task.
            (
                doc! { "expires_at": 1 },
                Some(
                    IndexOptions::builder()
                        .expire_after(std::time::Duration::from_secs(0))
                        .name("bootstrap_expires_at_ttl".to_string())
                        .build(),
                ),
            ),
            // Unique compound (space, agent) — one row per agent per space; the
            // PUT upsert keys on this, making eviction deterministic across
            // replicas instead of the per-pod arbitrary eviction.
            (
                doc! { "space": 1, "agent": 1 },
                Some(
                    IndexOptions::builder()
                        .unique(true)
                        .name("bootstrap_space_agent_unique".to_string())
                        .build(),
                ),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_declares_ttl_and_unique_indexes() {
        let names: Vec<_> = BootstrapEntryDoc::into_indices()
            .into_iter()
            .filter_map(|(_, o)| o.and_then(|x| x.name))
            .collect();
        assert!(
            names.iter().any(|n| n == "bootstrap_expires_at_ttl"),
            "missing TTL index, got {names:?}"
        );
        assert!(
            names.iter().any(|n| n == "bootstrap_space_agent_unique"),
            "missing unique index, got {names:?}"
        );
    }
}
