//! Production [`CommitmentAuthor`] for Phase-3 salvage (P3-5).
//!
//! When the salvage pass self-selects this node as a new holder for an
//! under-replicated blob, it must author a NOTARIZED placement intent — a
//! `custody-blob` REA commitment. That intent is Class A (notarized), so it goes
//! through the conductor (`content_store::create_rea_commitment`), NOT a local
//! SQL insert. Post-commit the DNA emits `ProjectionSignal::ReaCommitmentCommitted`
//! and `rea_projection` projects it into `rea_commitments` with `dht_anchor_hash`.
//! The next custody reconcile pass then sees `provider == self`, the blob missing
//! locally, and fetches the bytes — **salvage authors intent; reconcile moves
//! bytes** (no new fetch path).
//!
//! ## Sync trait over an async conductor call
//!
//! [`crate::reconcile::custody::CommitmentAuthor`] is SYNC (so `salvage_pass`
//! stays sync + unit-testable). [`conductor_writes::call_create_rea_commitment`]
//! is async. We bridge with the codebase-canonical
//! `tokio::task::block_in_place` + `Handle::block_on` pattern (mirrors
//! `services::epr_store`), which moves other tokio tasks off this worker thread
//! before driving the conductor future to completion — avoiding the
//! "cannot start a runtime from within a runtime" panic. The caller
//! (`P2PNode::run_salvage_pass` / the reconcile task) is already on a tokio
//! runtime, so a current `Handle` is always available there.

use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::reconcile::custody::CommitmentAuthor;
use crate::services::conductor_writes;

/// Authors notarized salvage `custody-blob` commitments via the conductor.
pub struct SalvageCommitmentAuthor {
    hc: Arc<HcClient>,
}

impl SalvageCommitmentAuthor {
    /// Construct with a live conductor handle (the `lamad`/`content_store` cell).
    pub fn new(hc: Arc<HcClient>) -> Self {
        Self { hc }
    }
}

/// Deterministic commitment id matching the seeder's custody-blob shape
/// (`seed-commitments.ts::buildCustodyCommitmentBody`):
/// `custody-blob-<sha256(provider|receiver|blob)[:16]>`. Re-authoring the same
/// (provider, receiver, blob) tuple yields the same id → idempotent at the
/// conductor (a duplicate is a no-op / 409, never a corrupt second row).
fn deterministic_custody_id(provider: &str, receiver: &str, blob_marker: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{provider}|{receiver}|{blob_marker}").as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("custody-blob-{}", &digest[..16])
}

impl CommitmentAuthor for SalvageCommitmentAuthor {
    fn author_custody_blob(
        &self,
        blob_marker: &str,
        provider: &str,
        receiver: &str,
    ) -> Result<(), StorageError> {
        // Build the canonical custody-blob input. Matches the seeder/HTTP shape
        // (action="custody-blob", resource_classified_as=[blob_marker], unit "B").
        let input = shefa_types::CreateReaCommitmentInput {
            id: deterministic_custody_id(provider, receiver, blob_marker),
            action: "custody-blob".to_string(),
            provider: provider.to_string(),
            receiver: receiver.to_string(),
            resource_classified_as: vec![blob_marker.to_string()],
            resource_quantity_value: None,
            resource_quantity_unit: Some("B".to_string()),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: Vec::new(),
            note: Some(format!(
                "salvage: {provider} self-selected to host {blob_marker} for {receiver}"
            )),
            metadata_json: Some(
                serde_json::json!({
                    "origin": "salvage",
                    "blobMarker": blob_marker,
                })
                .to_string(),
            ),
        };

        // Bridge sync → async with the canonical block_in_place + block_on pattern.
        let handle = tokio::runtime::Handle::try_current().map_err(|_| {
            StorageError::Internal(
                "salvage author: no tokio runtime handle (must be called from an async context)"
                    .to_string(),
            )
        })?;
        let hc = self.hc.clone();
        tokio::task::block_in_place(|| {
            handle.block_on(async move {
                conductor_writes::call_create_rea_commitment(&hc, &input).await
            })
        })
        .map(|_bytes| ())
    }
}

/// P3-6: Run one salvage pass against the local projection.
///
/// Builds the candidate pool from FRESH, opted-in `salvage_capacity` rows
/// (mapped to `PlacementCandidate`, agent_cid-keyed — the pool the XOR metric
/// ranks never crosses namespaces), adds self so this node can self-select,
/// constructs the MVP [`XorDistanceStrategy`], and invokes
/// [`crate::reconcile::custody::salvage_pass`] with the injected `author`.
///
/// **Safe no-op** when disabled (`salvage_capacity_enabled = false` → the pass
/// skips all authoring) or when the pool is empty (no under-replicated blob can
/// self-select). Authoring is the only side effect; the next custody reconcile
/// pass moves the bytes — **salvage authors intent; reconcile moves bytes** (no
/// new fetch path).
///
/// Lives here (not in `P2PNode`) because the production [`SalvageCommitmentAuthor`]
/// needs the conductor handle, which is threaded in the reconcile task — the same
/// place [`crate::services::conductor_commitment_author::ConductorCommitmentAuthor`]
/// is wired for the provide loop. Returns the
/// [`crate::reconcile::custody::SalvageOutcome`] for logging/metrics.
#[allow(clippy::too_many_arguments)]
pub fn run_salvage_pass(
    conn: &mut diesel::SqliteConnection,
    self_cid: &str,
    author: &dyn CommitmentAuthor,
    enabled: bool,
    target_replicas: usize,
    inventory_freshness_seconds: u64,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<crate::reconcile::custody::SalvageOutcome, StorageError> {
    use crate::reconcile::custody::{salvage_pass, SalvageConfig};
    use crate::reconcile::placement::{PlacementCandidate, XorDistanceStrategy};

    let fresh_after = (now - chrono::Duration::seconds(inventory_freshness_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let fresh_rows = crate::db::salvage_capacity::list_fresh(conn, &fresh_after)?;
    let mut candidates: Vec<PlacementCandidate> = fresh_rows
        .into_iter()
        .map(|r| PlacementCandidate {
            agent_cid: r.agent_cid,
            household_id: None,
            archetype: Some(r.archetype),
            spare_bytes: Some(r.spare_bytes.max(0) as u64),
        })
        .collect();
    // Include self so it can self-select even before its own capacity ad has
    // round-tripped back through gossip (first-tick convergence).
    if !candidates.iter().any(|c| c.agent_cid == self_cid) {
        candidates.push(PlacementCandidate::from_agent_cid(self_cid.to_string()));
    }

    let strategy = XorDistanceStrategy;
    let cfg = SalvageConfig {
        enabled,
        target_replicas,
        inventory_freshness_seconds,
    };
    salvage_pass(conn, self_cid, &strategy, author, &candidates, cfg, now)
}
