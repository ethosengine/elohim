//! Station 2 — a STANDING `custody-spool` commitment expands into one
//! `custody-blob` commitment per death witness the ward advertises.
//!
//! The ward's conductor is the thing that died, so the ward can author nothing.
//! The CUSTODIAN authors — on its own conductor, under a commitment it itself
//! authored (authorship IS the counter-signature). This pass is the expansion
//! step between those two facts:
//!
//! ```text
//! custody-spool(provider = self, receiver = ward)      standing consent (seeded once)
//!   × peer_blob_inventory rows advertised BY the ward   what the ward currently holds
//!   × local content row: metadata.kind == death-witness which of those is a witness
//!   → custody-blob(provider = self, receiver = ward, blob = sha256-…)   per-hash intent
//! ```
//!
//! Nothing new at the DHT: `custody-spool` and `custody-blob` are both the
//! existing elohim-DNA `Commitment` entry type, authored through
//! `content_store::create_rea_commitment`. **This pass authors intent; the
//! existing custody reconcile sweep moves the bytes** — no new fetch path, and
//! the `serve-blob` economic event the fetch already emits is the receipt.
//!
//! ## Identity discipline
//!
//! Provider is ALWAYS this peer's resolved holochain `agent_cid` (`uhCAk…`, via
//! [`crate::services::salvage_commitment_author::resolve_self_agent_cid`]); an
//! unresolvable self SKIPS the tick rather than writing a transport id. The ward
//! is the commitment's `receiver` — also an `agent_cid`. Agent ids are NEVER
//! string-compared against transport ids: the ward's advertised inventory is
//! found by first RESOLVING the ward's transport identities
//! ([`ward_inventory_peer_ids`] — `peer_identity_bindings` +
//! `peer_transport_manifest`), and a ward whose identity resolves to nothing that
//! ever advertised is skipped with a logged reason, never guessed at.
//!
//! Reading `peer_identity_bindings` here is the ROUTING cut, not the attribution
//! cut: the binding only decides WHICH advertised bytes are the ward's, while the
//! `receiver` we credit comes from the already-notarized `custody-spool` row and
//! the bytes verify by hash. See `db/peer_identity_bindings.rs`.
//!
//! ## Bounded work (uncancellable conductor call)
//!
//! `author_custody_blob` bridges into an uncancellable conductor round-trip, so
//! the per-tick authoring budget is computed BEFORE the first call:
//! [`per_tick_cap`] derives it from the commitment's `bounds.atomsPerHour`
//! (default [`DEFAULT_ATOMS_PER_HOUR`]) and the sweep cadence. Over-budget
//! witnesses are a WITNESSED refusal ([`SkipReason::BoundsExceeded`] + a log
//! line), never a silent drop; the next tick picks them up.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::SqliteConnection;
use tracing::{info, warn};

use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::reconcile::custody::CommitmentAuthor;
use crate::services::conductor_writes;
use crate::services::rea_commitment_service::deterministic_custody_id;

pub use crate::services::rea_commitment_service::{
    deterministic_spool_custody_id, spool_classification, SPOOL_CUSTODY_ACTION,
};

/// `metadata.kind` marking a content row as an ark death witness (written by
/// [`crate::services::spool_ingest::witness_content_input`]).
pub const DEATH_WITNESS_KIND: &str = "death-witness";

/// Authoring budget assumed when a spool commitment declares no
/// `bounds.atomsPerHour` — the seeder's default.
pub const DEFAULT_ATOMS_PER_HOUR: u32 = 120;

/// Commitment states that mean "this pledge is retired". Everything else counts
/// as live: the DNA mints `created`, the accept/activate paths move it through
/// `proposed`/`accepted`/`activated`/`active`, and gating on a single state
/// string here would silently disable the whole station.
const RETIRED_STATES: [&str; 5] = [
    "cancelled",
    "superseded",
    "revoked",
    "completed",
    "rejected",
];

// ─────────────────────────────────────────────────────────────────────────────
// Pass result
// ─────────────────────────────────────────────────────────────────────────────

/// Why one candidate (or one whole ward) was not authored this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// The ward's `agent_cid` resolved to no transport identity that has ever
    /// advertised inventory here. Never guessed at — skipped and logged.
    WardPeerUnresolved,
    /// The spool row's `resource_classified_as` is not `spool:witness:<receiver>`.
    ClassificationMismatch,
    /// The per-tick authoring budget (`bounds.atomsPerHour`) or the byte budget
    /// (`bounds.maxBytes`) is spent. A witnessed refusal — retried next tick.
    BoundsExceeded,
}

impl SkipReason {
    /// Stable log/metric token.
    pub fn as_str(self) -> &'static str {
        match self {
            SkipReason::WardPeerUnresolved => "ward-peer-unresolved",
            SkipReason::ClassificationMismatch => "classification-mismatch",
            SkipReason::BoundsExceeded => "bounds-exceeded",
        }
    }
}

/// One reasoned refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpoolCustodySkip {
    /// Ward (`agent_cid`) whose spool commitment was being expanded.
    pub ward: String,
    /// Blob marker (`sha256-<hex>`) when the skip is per-witness; `None` when
    /// the whole ward was skipped.
    pub blob: Option<String>,
    pub reason: SkipReason,
}

/// Outcome of one expansion tick.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpoolCustodyPass {
    /// Blob markers a `custody-blob` was authored for this tick.
    pub authored: Vec<String>,
    /// Witnesses already covered by an existing `custody-blob` (idempotent).
    pub already: usize,
    /// The tick authored nothing because this peer's `agent_cid` is unresolvable.
    pub skipped_no_self: bool,
    /// Reasoned refusals (bounds, unresolvable ward, malformed row).
    pub skipped: Vec<SpoolCustodySkip>,
}

impl SpoolCustodyPass {
    /// Refusals carrying `reason`.
    pub fn skips_with(&self, reason: SkipReason) -> Vec<&SpoolCustodySkip> {
        self.skipped.iter().filter(|s| s.reason == reason).collect()
    }
}

/// Per-tick configuration. `tick_seconds` is the sweep cadence the dispatcher
/// runs at (`Config::custody_sweep_seconds`) and is what turns the commitment's
/// hourly atom budget into a per-tick cap.
#[derive(Debug, Clone, Copy)]
pub struct SpoolCustodyConfig {
    pub tick_seconds: u64,
}

impl Default for SpoolCustodyConfig {
    fn default() -> Self {
        Self { tick_seconds: 120 }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounds
// ─────────────────────────────────────────────────────────────────────────────

/// Bounds a `custody-spool` commitment declares in `metadata_json.bounds`.
///
/// Wire shape is the seeder's camelCase (`maxBytes` / `atomsPerHour` /
/// `retentionDays`); snake_case aliases are accepted so a hand-authored row is
/// not silently unbounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpoolBounds {
    /// Byte budget this tick may commit to. `None` = undeclared (unbounded).
    pub max_bytes: Option<u64>,
    /// Witnesses per hour this custodian may adopt for the ward.
    pub atoms_per_hour: u32,
    /// Recorded, NOT enforced in this slice (S1 owns retention).
    pub retention_days: Option<u32>,
}

impl Default for SpoolBounds {
    fn default() -> Self {
        Self {
            max_bytes: None,
            atoms_per_hour: DEFAULT_ATOMS_PER_HOUR,
            retention_days: None,
        }
    }
}

/// Parse `metadata_json.bounds` off a spool commitment row. Missing/unparseable
/// metadata yields the defaults (never an error — a malformed row must not stall
/// the station, it just runs at the default budget).
pub fn parse_spool_bounds(metadata_json: Option<&str>) -> SpoolBounds {
    let Some(raw) = metadata_json else {
        return SpoolBounds::default();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return SpoolBounds::default();
    };
    let Some(bounds) = value.get("bounds") else {
        return SpoolBounds::default();
    };
    let read_u64 = |camel: &str, snake: &str| -> Option<u64> {
        bounds
            .get(camel)
            .or_else(|| bounds.get(snake))
            .and_then(serde_json::Value::as_u64)
    };
    SpoolBounds {
        max_bytes: read_u64("maxBytes", "max_bytes"),
        atoms_per_hour: read_u64("atomsPerHour", "atoms_per_hour")
            .map(|v| u32::try_from(v).unwrap_or(u32::MAX))
            .unwrap_or(DEFAULT_ATOMS_PER_HOUR),
        retention_days: read_u64("retentionDays", "retention_days")
            .map(|v| u32::try_from(v).unwrap_or(u32::MAX)),
    }
}

/// How many witnesses one tick may author, derived from the hourly atom budget
/// and the sweep cadence.
///
/// A declared budget always leaves at least ONE authoring per tick (a 120/h
/// budget on a 15 s sweep is 0.5/tick — flooring that to zero would deadlock the
/// station), so the hourly rate is the ceiling over hours, not a per-tick floor.
/// `atoms_per_hour == 0` is an explicit refusal and yields 0.
pub fn per_tick_cap(atoms_per_hour: u32, tick_seconds: u64) -> usize {
    if atoms_per_hour == 0 {
        return 0;
    }
    let per_tick = u64::from(atoms_per_hour).saturating_mul(tick_seconds.max(1)) / 3600;
    per_tick.max(1) as usize
}

// ─────────────────────────────────────────────────────────────────────────────
// The pass
// ─────────────────────────────────────────────────────────────────────────────

/// A live `custody-spool` row this peer provides.
struct SpoolPledge {
    ward: String,
    bounds: SpoolBounds,
}

/// Resolve every inventory `peer_id` under which the ward may have advertised.
///
/// Three sources, all exact matches — never a cross-namespace guess:
/// 1. the ward's own `agent_cid` (inventory published by an agent-keyed peer),
/// 2. `peer_identity_bindings` active rows for the ward (routing cut),
/// 3. `peer_transport_manifest` (`libp2p_peer_id` / `iroh_node_id`), read
///    straight off the table so this works in every feature build.
///
/// The ward's own cid is always first; `resolved` reports how many TRANSPORT
/// identities were resolved, so the caller can tell "ward advertises nothing"
/// from "we cannot tell which peer is the ward".
pub fn ward_inventory_peer_ids(
    conn: &mut SqliteConnection,
    ward: &str,
    now_iso: &str,
) -> Result<Vec<String>, StorageError> {
    use crate::db::diesel_schema::peer_transport_manifest as ptm;

    let mut ids: Vec<String> = vec![ward.to_string()];

    for row in crate::db::peer_identity_bindings::list_active_for_agent(conn, ward, now_iso)? {
        ids.push(row.peer_id);
    }

    let manifest: Option<(Option<String>, Option<String>)> = ptm::table
        .filter(ptm::agent_cid.eq(ward))
        .select((ptm::libp2p_peer_id, ptm::iroh_node_id))
        .first(conn)
        .optional()
        .map_err(|e| {
            StorageError::Database(format!("spool custody: ward transport lookup: {e}"))
        })?;
    if let Some((libp2p_peer_id, iroh_node_id)) = manifest {
        ids.extend(libp2p_peer_id);
        ids.extend(iroh_node_id);
    }

    let mut seen = HashSet::new();
    ids.retain(|id| !id.is_empty() && seen.insert(id.clone()));
    Ok(ids)
}

/// Blob markers advertised under any of `peer_ids`.
fn advertised_blobs(
    conn: &mut SqliteConnection,
    peer_ids: &[String],
) -> Result<HashSet<String>, StorageError> {
    use crate::db::diesel_schema::peer_blob_inventory as pbi;
    let rows: Vec<String> = pbi::table
        .filter(pbi::peer_id.eq_any(peer_ids))
        .select(pbi::blob_hash)
        .load(conn)
        .map_err(|e| StorageError::Database(format!("spool custody: advertised blobs: {e}")))?;
    Ok(rows.into_iter().collect())
}

/// Death-witness content rows held locally, as `blob_hash -> size_bytes`.
///
/// This is the ONLY qualifier for "is that advertised blob a death witness":
/// `peer_blob_inventory` carries no metadata column (peer_id, blob_hash,
/// last_seen_at, source, sequence, blake3_hash, transport_affinity) and the
/// gossip `BlobHint` is consumed by the fetch prioritizer without being
/// persisted — so there is no inventory-side marker to fall back to. A witness
/// this custodian has no content row for is invisible to the pass until the row
/// arrives (correct-but-dormant, never a wrong custody).
///
/// Scanning is bounded by `content_type = 'issue-report'` rather than by the
/// advertised set, so the query never grows an `IN (…)` list the size of a
/// peer's whole inventory.
fn local_death_witnesses(
    conn: &mut SqliteConnection,
) -> Result<HashMap<String, Option<u64>>, StorageError> {
    use crate::db::diesel_schema::content;
    let rows: Vec<(Option<String>, Option<String>, Option<i32>)> = content::table
        .filter(content::content_type.eq("issue-report"))
        .filter(content::blob_hash.is_not_null())
        .select((
            content::blob_hash,
            content::metadata_json,
            content::content_size_bytes,
        ))
        .load(conn)
        .map_err(|e| StorageError::Database(format!("spool custody: witness rows: {e}")))?;

    let mut witnesses = HashMap::new();
    for (blob_hash, metadata_json, size) in rows {
        let (Some(blob_hash), Some(metadata_json)) = (blob_hash, metadata_json) else {
            continue;
        };
        let is_witness = serde_json::from_str::<serde_json::Value>(&metadata_json)
            .ok()
            .and_then(|m| {
                m.get("kind")
                    .and_then(serde_json::Value::as_str)
                    .map(|k| k == DEATH_WITNESS_KIND)
            })
            .unwrap_or(false);
        if is_witness {
            witnesses.insert(blob_hash, size.map(|s| s.max(0) as u64));
        }
    }
    Ok(witnesses)
}

/// Live `custody-spool` pledges this peer PROVIDES.
fn self_spool_pledges(
    conn: &mut SqliteConnection,
    self_agent: &str,
) -> Result<Vec<SpoolPledge>, StorageError> {
    use crate::db::diesel_schema::rea_commitments as rc;
    use crate::db::models::ReaCommitment;

    let rows: Vec<ReaCommitment> = rc::table
        .filter(rc::action.eq(SPOOL_CUSTODY_ACTION))
        .filter(rc::provider.eq(self_agent))
        .filter(rc::finished.eq(0))
        .order_by(rc::id.asc())
        .load(conn)
        .map_err(|e| StorageError::Database(format!("spool custody: load pledges: {e}")))?;

    Ok(rows
        .into_iter()
        .filter(|row| !RETIRED_STATES.contains(&row.state.as_str()))
        .map(|row| SpoolPledge {
            ward: row.receiver.clone(),
            bounds: parse_spool_bounds(row.metadata_json.as_deref()),
        })
        .collect())
}

/// Blob markers this peer already PROVIDES a `custody-blob` for.
fn self_custody_blob_markers(
    conn: &mut SqliteConnection,
    self_agent: &str,
) -> Result<HashSet<String>, StorageError> {
    use crate::db::diesel_schema::rea_commitments as rc;
    use crate::db::models::ReaCommitment;

    let rows: Vec<ReaCommitment> = rc::table
        .filter(rc::action.eq("custody-blob"))
        .filter(rc::provider.eq(self_agent))
        .load(conn)
        .map_err(|e| StorageError::Database(format!("spool custody: load custody rows: {e}")))?;
    Ok(rows
        .iter()
        .filter_map(ReaCommitment::primary_classification)
        .collect())
}

/// Run ONE expansion tick against the local projection.
///
/// Pure of transport and of the conductor: every write goes through the injected
/// [`CommitmentAuthor`], so the whole station is unit-testable with a recording
/// author (the salvage precedent) and never dials a live conductor from a test.
///
/// Safe no-op when this peer holds no `custody-spool` pledge, when the ward
/// advertises nothing, or when no advertised blob is a known death witness.
pub fn run_spool_custody_pass(
    conn: &mut SqliteConnection,
    self_agent_cid: Option<&str>,
    author: &dyn CommitmentAuthor,
    cfg: SpoolCustodyConfig,
    now: DateTime<Utc>,
) -> Result<SpoolCustodyPass, StorageError> {
    let mut pass = SpoolCustodyPass::default();

    // Never write a transport-id provider: an unresolvable self skips the tick.
    let self_agent = match self_agent_cid {
        Some(cid) if crate::identity_namespace::is_agent_cid(cid) => cid,
        _ => {
            pass.skipped_no_self = true;
            return Ok(pass);
        }
    };

    let pledges = self_spool_pledges(conn, self_agent)?;
    if pledges.is_empty() {
        return Ok(pass);
    }

    crate::identity_namespace::observe_agent_cid_write(
        "rea_commitments.provider",
        Some(self_agent),
    );

    let now_iso = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let witnesses = local_death_witnesses(conn)?;
    let mut covered = self_custody_blob_markers(conn, self_agent)?;

    for pledge in pledges {
        let peer_ids = ward_inventory_peer_ids(conn, &pledge.ward, &now_iso)?;
        let advertised = advertised_blobs(conn, &peer_ids)?;
        if advertised.is_empty() {
            // Either the ward advertises nothing, or its identity resolves to no
            // peer we have heard from. Both are honest skips — the pass NEVER
            // falls back to "any peer's inventory" to find the ward's bytes.
            if peer_ids.len() == 1 {
                warn!(
                    target: "elohim_storage::spool_custody",
                    ward = %pledge.ward,
                    reason = SkipReason::WardPeerUnresolved.as_str(),
                    "spool custody: ward has no resolved transport identity and no \
                     agent-keyed inventory — skipping (never guessing which peer is the ward)"
                );
                pass.skipped.push(SpoolCustodySkip {
                    ward: pledge.ward.clone(),
                    blob: None,
                    reason: SkipReason::WardPeerUnresolved,
                });
            }
            continue;
        }

        // Deterministic candidate order so the per-tick cap always spends on the
        // same witnesses across peers and restarts.
        let candidates: BTreeSet<&String> = advertised
            .iter()
            .filter(|blob| witnesses.contains_key(*blob))
            .collect();

        // Bounded work: the whole budget is computed BEFORE the first
        // (uncancellable) conductor call.
        let mut remaining = per_tick_cap(pledge.bounds.atoms_per_hour, cfg.tick_seconds);
        let mut bytes_left = pledge.bounds.max_bytes;

        for blob in candidates {
            if covered.contains(blob) {
                pass.already += 1;
                continue;
            }
            let size = witnesses.get(blob).copied().flatten().unwrap_or(0);
            let over_bytes = bytes_left.is_some_and(|left| size > left);
            if remaining == 0 || over_bytes {
                warn!(
                    target: "elohim_storage::spool_custody",
                    ward = %pledge.ward,
                    blob = %blob,
                    atoms_per_hour = pledge.bounds.atoms_per_hour,
                    max_bytes = ?pledge.bounds.max_bytes,
                    reason = SkipReason::BoundsExceeded.as_str(),
                    "spool custody: authoring budget spent — refusing this witness this tick \
                     (witnessed refusal; retried next tick)"
                );
                pass.skipped.push(SpoolCustodySkip {
                    ward: pledge.ward.clone(),
                    blob: Some(blob.clone()),
                    reason: SkipReason::BoundsExceeded,
                });
                continue;
            }

            author.author_custody_blob(blob, self_agent, &pledge.ward)?;
            remaining -= 1;
            bytes_left = bytes_left.map(|left| left.saturating_sub(size));
            covered.insert(blob.clone());
            pass.authored.push(blob.clone());
            info!(
                target: "elohim_storage::spool_custody",
                ward = %pledge.ward,
                blob = %blob,
                commitment_id = %deterministic_custody_id(self_agent, &pledge.ward, blob),
                "spool custody: authored custody-blob for a ward's death witness"
            );
        }
    }

    Ok(pass)
}

// ─────────────────────────────────────────────────────────────────────────────
// Production author (conductor-backed)
// ─────────────────────────────────────────────────────────────────────────────

/// Production [`CommitmentAuthor`] + dispatcher for the spool-custody station.
///
/// Mirrors [`crate::services::salvage_commitment_author::SalvageCommitmentAuthor`]
/// exactly — same sync-trait-over-async-conductor bridge
/// (`block_in_place` + `Handle::block_on`), same self-identity resolver — and
/// differs only in the commitment's provenance metadata (`origin: spool-custody`).
pub struct SpoolCustodyAuthor {
    hc: Arc<HcClient>,
}

impl SpoolCustodyAuthor {
    /// Construct with a live conductor handle (the `lamad`/`content_store` cell).
    pub fn new(hc: Arc<HcClient>) -> Self {
        Self { hc }
    }

    /// This peer's `agent_cid` to WRITE as `provider`, or `None` (skip the tick).
    pub fn resolve_self_agent_cid(&self, conn: &mut SqliteConnection) -> Option<String> {
        crate::services::salvage_commitment_author::resolve_self_agent_cid(conn, &self.hc)
    }

    /// One tick on THIS peer, authoring through its own conductor.
    pub fn run_once(
        &self,
        conn: &mut SqliteConnection,
        cfg: SpoolCustodyConfig,
        now: DateTime<Utc>,
    ) -> Result<SpoolCustodyPass, StorageError> {
        let self_agent = self.resolve_self_agent_cid(conn);
        run_spool_custody_pass(conn, self_agent.as_deref(), self, cfg, now)
    }
}

impl CommitmentAuthor for SpoolCustodyAuthor {
    fn author_custody_blob(
        &self,
        blob_marker: &str,
        provider: &str,
        receiver: &str,
    ) -> Result<(), StorageError> {
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
                "spool custody: {provider} holds {receiver}'s witness {blob_marker}"
            )),
            metadata_json: Some(
                serde_json::json!({
                    "origin": "spool-custody",
                    "blobMarker": blob_marker,
                    "ward": receiver,
                })
                .to_string(),
            ),
        };

        let handle = tokio::runtime::Handle::try_current().map_err(|_| {
            StorageError::Internal(
                "spool custody author: no tokio runtime handle (must be called from an async context)"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::content_diesel::{create_content, CreateContentInput};
    use crate::db::models::{NewPeerIdentityBindingRow, NewReaCommitment};
    use crate::db::AppContext;
    use crate::p2p::binding_proof_wire::BindingProofStatus;
    use crate::test_util::test_pool;

    const SELF_AGENT: &str = "uhCAk-matthew";
    const WARD: &str = "uhCAk-jessica";
    const WITNESS_A: &str =
        "sha256-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const WITNESS_B: &str =
        "sha256-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const NOT_A_WITNESS: &str =
        "sha256-cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    fn iso(at: DateTime<Utc>) -> String {
        at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }

    /// Records what a pass authored, exactly like the salvage tests' harness —
    /// no conductor is ever dialled from a unit test.
    #[derive(Default)]
    struct RecordingAuthor {
        authored: std::sync::Mutex<Vec<(String, String, String)>>,
    }

    impl RecordingAuthor {
        fn tuples(&self) -> Vec<(String, String, String)> {
            self.authored.lock().unwrap().clone()
        }
    }

    impl CommitmentAuthor for RecordingAuthor {
        fn author_custody_blob(
            &self,
            blob_marker: &str,
            provider: &str,
            receiver: &str,
        ) -> Result<(), StorageError> {
            self.authored.lock().unwrap().push((
                blob_marker.to_string(),
                provider.to_string(),
                receiver.to_string(),
            ));
            Ok(())
        }
    }

    fn seed_spool_commitment(
        conn: &mut SqliteConnection,
        provider: &str,
        ward: &str,
        bounds_json: Option<&str>,
    ) {
        use crate::db::diesel_schema::rea_commitments;
        let id = deterministic_spool_custody_id(provider, ward, ward);
        let classification = format!("[\"{}\"]", spool_classification(ward));
        diesel::insert_into(rea_commitments::table)
            .values(&NewReaCommitment {
                id: &id,
                h_app_id: "lamad",
                action: SPOOL_CUSTODY_ACTION,
                provider,
                receiver: ward,
                resource_conforms_to: None,
                resource_classified_as: Some(&classification),
                resource_quantity_value: Some(64.0),
                resource_quantity_unit: Some("B"),
                effort_quantity_value: None,
                effort_quantity_unit: None,
                has_beginning: None,
                has_end: None,
                due: None,
                clause_of: None,
                in_scope_of: None,
                medium_of_exchange_id: None,
                // The DNA mints `created`; the pass must treat that as live.
                state: "created",
                finished: 0,
                note: None,
                metadata_json: bounds_json,
                dht_anchor_hash: Some("uhCkk-spool"),
            })
            .execute(conn)
            .expect("insert custody-spool");
    }

    fn seed_witness_row(conn: &mut SqliteConnection, id: &str, blob_hash: &str, kind: &str) {
        create_content(
            conn,
            &AppContext::default_lamad(),
            CreateContentInput {
                id: id.to_string(),
                title: format!("death witness {id}"),
                description: None,
                content_type: "issue-report".to_string(),
                content_format: "json".to_string(),
                blob_hash: Some(blob_hash.to_string()),
                blob_cid: None,
                content_size_bytes: Some(1024),
                metadata_json: Some(serde_json::json!({ "kind": kind }).to_string()),
                reach: "private".to_string(),
                created_by: Some(WARD.to_string()),
                tags: Vec::new(),
                content_body: None,
                dht_anchor_hash: None,
            },
        )
        .expect("insert witness content row");
    }

    fn seed_inventory(conn: &mut SqliteConnection, peer_id: &str, blobs: &[&str], at: &str) {
        let owned: Vec<String> = blobs.iter().map(|b| b.to_string()).collect();
        crate::db::peer_blob_inventory::apply_snapshot(conn, peer_id, &owned, 1, at)
            .expect("apply inventory snapshot");
    }

    fn seed_binding(conn: &mut SqliteConnection, peer_id: &str, agent_cid: &str, at: &str) {
        crate::db::peer_identity_bindings::upsert(
            conn,
            &NewPeerIdentityBindingRow {
                peer_id: peer_id.to_string(),
                agent_cid: agent_cid.to_string(),
                dht_anchor_hash: format!("uhCkk-{peer_id}"),
                valid_from: at.to_string(),
                valid_until: None,
                observed_at: at.to_string(),
                source: "handshake".to_string(),
                device_archetype: "node".to_string(),
                superseded_by: None,
                signature: String::new(),
                proof_status: BindingProofStatus::unverified(),
            },
        )
        .expect("insert binding");
    }

    /// The cross-language contract with the seeder's `buildSpoolCustodyInput`.
    #[test]
    fn pins_the_cross_language_spool_custody_id_vector() {
        assert_eq!(
            deterministic_spool_custody_id("uhCAkA", "uhCAkB", "uhCAkB"),
            "custody-spool-4c267bbf6ea97775"
        );
        assert_eq!(spool_classification("uhCAkB"), "spool:witness:uhCAkB");
        assert_eq!(SPOOL_CUSTODY_ACTION, "custody-spool");
    }

    #[test]
    fn expands_one_custody_blob_per_advertised_witness() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let at = now();
        seed_spool_commitment(&mut conn, SELF_AGENT, WARD, None);
        seed_witness_row(&mut conn, "witness-a", WITNESS_A, DEATH_WITNESS_KIND);
        seed_witness_row(&mut conn, "witness-b", WITNESS_B, DEATH_WITNESS_KIND);
        seed_inventory(&mut conn, WARD, &[WITNESS_A, WITNESS_B], &iso(at));

        let author = RecordingAuthor::default();
        // 120 atoms/h over a 3600 s tick = 120 per tick — both witnesses fit.
        let pass = run_spool_custody_pass(
            &mut conn,
            Some(SELF_AGENT),
            &author,
            SpoolCustodyConfig { tick_seconds: 3600 },
            at,
        )
        .unwrap();

        assert_eq!(pass.authored.len(), 2, "one custody-blob per witness");
        assert!(
            pass.skipped.is_empty(),
            "nothing refused: {:?}",
            pass.skipped
        );
        let mut tuples = author.tuples();
        tuples.sort();
        assert_eq!(
            tuples,
            vec![
                (
                    WITNESS_A.to_string(),
                    SELF_AGENT.to_string(),
                    WARD.to_string()
                ),
                (
                    WITNESS_B.to_string(),
                    SELF_AGENT.to_string(),
                    WARD.to_string()
                ),
            ],
            "provider = self (agent_cid), receiver = the ward"
        );
    }

    #[test]
    fn is_idempotent_across_ticks() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let at = now();
        seed_spool_commitment(&mut conn, SELF_AGENT, WARD, None);
        seed_witness_row(&mut conn, "witness-a", WITNESS_A, DEATH_WITNESS_KIND);
        seed_inventory(&mut conn, WARD, &[WITNESS_A], &iso(at));

        let author = RecordingAuthor::default();
        let cfg = SpoolCustodyConfig { tick_seconds: 3600 };
        let first = run_spool_custody_pass(&mut conn, Some(SELF_AGENT), &author, cfg, at).unwrap();
        assert_eq!(first.authored, vec![WITNESS_A.to_string()]);

        // Project what the conductor round-trip would land, then re-run.
        use crate::db::diesel_schema::rea_commitments;
        let classification = format!("[\"{WITNESS_A}\"]");
        let id = deterministic_custody_id(SELF_AGENT, WARD, WITNESS_A);
        diesel::insert_into(rea_commitments::table)
            .values(&NewReaCommitment {
                id: &id,
                h_app_id: "lamad",
                action: "custody-blob",
                provider: SELF_AGENT,
                receiver: WARD,
                resource_conforms_to: None,
                resource_classified_as: Some(&classification),
                resource_quantity_value: None,
                resource_quantity_unit: Some("B"),
                effort_quantity_value: None,
                effort_quantity_unit: None,
                has_beginning: None,
                has_end: None,
                due: None,
                clause_of: None,
                in_scope_of: None,
                medium_of_exchange_id: None,
                state: "created",
                finished: 0,
                note: None,
                metadata_json: None,
                dht_anchor_hash: Some("uhCkk-custody"),
            })
            .execute(&mut conn)
            .unwrap();

        let second = run_spool_custody_pass(&mut conn, Some(SELF_AGENT), &author, cfg, at).unwrap();
        assert!(second.authored.is_empty(), "no duplicate authoring");
        assert_eq!(
            second.already, 1,
            "the existing pledge is counted, not re-authored"
        );
        assert_eq!(
            author.tuples().len(),
            1,
            "the conductor is called exactly once"
        );
    }

    #[test]
    fn skips_when_self_agent_unresolved() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let at = now();
        seed_spool_commitment(&mut conn, SELF_AGENT, WARD, None);
        seed_witness_row(&mut conn, "witness-a", WITNESS_A, DEATH_WITNESS_KIND);
        seed_inventory(&mut conn, WARD, &[WITNESS_A], &iso(at));

        let author = RecordingAuthor::default();
        let cfg = SpoolCustodyConfig { tick_seconds: 3600 };

        let unresolved = run_spool_custody_pass(&mut conn, None, &author, cfg, at).unwrap();
        assert!(unresolved.skipped_no_self);
        assert!(unresolved.authored.is_empty());

        // A transport id offered where an agent_cid was expected is refused too —
        // a `12D3Koo…` provider could never join humans.agent_pub_key.
        let transport =
            run_spool_custody_pass(&mut conn, Some("12D3KooTransportSelfId"), &author, cfg, at)
                .unwrap();
        assert!(transport.skipped_no_self);
        assert!(
            author.tuples().is_empty(),
            "no transport-id provider row may ever be authored"
        );
    }

    #[test]
    fn refuses_beyond_atoms_per_hour_with_a_logged_reason() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let at = now();
        // 120 atoms/h on a 15 s sweep → cap 1 per tick.
        let bounds = serde_json::json!({
            "kind": "custody-spool",
            "bounds": { "maxBytes": 64 << 20, "atomsPerHour": 120, "retentionDays": 90 }
        })
        .to_string();
        seed_spool_commitment(&mut conn, SELF_AGENT, WARD, Some(&bounds));
        seed_witness_row(&mut conn, "witness-a", WITNESS_A, DEATH_WITNESS_KIND);
        seed_witness_row(&mut conn, "witness-b", WITNESS_B, DEATH_WITNESS_KIND);
        seed_inventory(&mut conn, WARD, &[WITNESS_A, WITNESS_B], &iso(at));

        let author = RecordingAuthor::default();
        let pass = run_spool_custody_pass(
            &mut conn,
            Some(SELF_AGENT),
            &author,
            SpoolCustodyConfig { tick_seconds: 15 },
            at,
        )
        .unwrap();

        assert_eq!(pass.authored.len(), 1, "the per-tick cap bounds the work");
        let refusals = pass.skips_with(SkipReason::BoundsExceeded);
        assert_eq!(
            refusals.len(),
            1,
            "the over-budget witness is a WITNESSED refusal"
        );
        assert_eq!(refusals[0].ward, WARD);
        assert!(refusals[0].blob.is_some());
        assert_eq!(
            author.tuples().len(),
            1,
            "the conductor is called at most cap times"
        );

        // A zero atom budget is an explicit refusal to author at all.
        let zero = serde_json::json!({ "bounds": { "atomsPerHour": 0 } }).to_string();
        assert_eq!(parse_spool_bounds(Some(&zero)).atoms_per_hour, 0);
        assert_eq!(per_tick_cap(0, 3600), 0);
        assert_eq!(
            per_tick_cap(120, 15),
            1,
            "a declared budget never floors to zero"
        );
        assert_eq!(per_tick_cap(120, 3600), 120);
    }

    #[test]
    fn ignores_blobs_the_ward_did_not_advertise() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let at = now();
        seed_spool_commitment(&mut conn, SELF_AGENT, WARD, None);
        // A witness the ward holds…
        seed_witness_row(&mut conn, "witness-a", WITNESS_A, DEATH_WITNESS_KIND);
        // …a witness only SOMEONE ELSE advertises…
        seed_witness_row(&mut conn, "witness-b", WITNESS_B, DEATH_WITNESS_KIND);
        // …and a blob the ward advertises that is not a witness at all.
        seed_witness_row(&mut conn, "not-a-witness", NOT_A_WITNESS, "issue");
        seed_inventory(&mut conn, WARD, &[WITNESS_A, NOT_A_WITNESS], &iso(at));
        seed_inventory(&mut conn, "uhCAk-stranger", &[WITNESS_B], &iso(at));

        let author = RecordingAuthor::default();
        let pass = run_spool_custody_pass(
            &mut conn,
            Some(SELF_AGENT),
            &author,
            SpoolCustodyConfig { tick_seconds: 3600 },
            at,
        )
        .unwrap();

        assert_eq!(
            pass.authored,
            vec![WITNESS_A.to_string()],
            "only the ward's OWN advertised death witness is adopted"
        );
    }

    #[test]
    fn resolves_the_wards_advertisement_through_an_identity_binding() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let at = now();
        seed_spool_commitment(&mut conn, SELF_AGENT, WARD, None);
        seed_witness_row(&mut conn, "witness-a", WITNESS_A, DEATH_WITNESS_KIND);
        // The ward advertises under its libp2p transport id, never its agent key.
        seed_inventory(&mut conn, "12D3KooWard", &[WITNESS_A], &iso(at));

        let author = RecordingAuthor::default();
        let cfg = SpoolCustodyConfig { tick_seconds: 3600 };

        let unbound =
            run_spool_custody_pass(&mut conn, Some(SELF_AGENT), &author, cfg, at).unwrap();
        assert!(
            unbound.authored.is_empty(),
            "without a binding the transport id is NEVER guessed to be the ward"
        );
        assert_eq!(
            unbound.skips_with(SkipReason::WardPeerUnresolved).len(),
            1,
            "the unresolvable ward is skipped with a reason"
        );

        seed_binding(&mut conn, "12D3KooWard", WARD, &iso(at));
        let bound = run_spool_custody_pass(&mut conn, Some(SELF_AGENT), &author, cfg, at).unwrap();
        assert_eq!(
            bound.authored,
            vec![WITNESS_A.to_string()],
            "the binding resolves the ward's advertisement"
        );
    }

    #[test]
    fn a_peer_with_no_spool_pledge_authors_nothing() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let at = now();
        seed_witness_row(&mut conn, "witness-a", WITNESS_A, DEATH_WITNESS_KIND);
        seed_inventory(&mut conn, WARD, &[WITNESS_A], &iso(at));

        let author = RecordingAuthor::default();
        let pass = run_spool_custody_pass(
            &mut conn,
            Some(SELF_AGENT),
            &author,
            SpoolCustodyConfig { tick_seconds: 3600 },
            at,
        )
        .unwrap();

        assert_eq!(
            pass,
            SpoolCustodyPass::default(),
            "safe no-op without consent"
        );
    }

    #[test]
    fn a_retired_pledge_stops_expanding() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let at = now();
        seed_spool_commitment(&mut conn, SELF_AGENT, WARD, None);
        seed_witness_row(&mut conn, "witness-a", WITNESS_A, DEATH_WITNESS_KIND);
        seed_inventory(&mut conn, WARD, &[WITNESS_A], &iso(at));

        use crate::db::diesel_schema::rea_commitments as rc;
        diesel::update(rc::table.filter(rc::action.eq(SPOOL_CUSTODY_ACTION)))
            .set(rc::state.eq("cancelled"))
            .execute(&mut conn)
            .unwrap();

        let author = RecordingAuthor::default();
        let pass = run_spool_custody_pass(
            &mut conn,
            Some(SELF_AGENT),
            &author,
            SpoolCustodyConfig { tick_seconds: 3600 },
            at,
        )
        .unwrap();
        assert!(
            pass.authored.is_empty(),
            "a cancelled pledge authors nothing"
        );
    }

    #[test]
    fn bounds_default_when_metadata_is_absent_or_malformed() {
        assert_eq!(parse_spool_bounds(None), SpoolBounds::default());
        assert_eq!(parse_spool_bounds(Some("{oops")), SpoolBounds::default());
        assert_eq!(parse_spool_bounds(Some("{}")), SpoolBounds::default());
        let snake = serde_json::json!({
            "bounds": { "max_bytes": 4096, "atoms_per_hour": 7, "retention_days": 30 }
        })
        .to_string();
        let parsed = parse_spool_bounds(Some(&snake));
        assert_eq!(parsed.max_bytes, Some(4096));
        assert_eq!(parsed.atoms_per_hour, 7);
        assert_eq!(parsed.retention_days, Some(30));
    }
}
