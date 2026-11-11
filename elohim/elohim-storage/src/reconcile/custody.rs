//! Custody reconciliation controller.
//!
//! Multi-trigger reconcile pass:
//! - Gossip arrival (`BlobInventorySnapshot` / `BlobInventoryDelta` for peer X)
//! - Connection event (`ConnectionEstablished` for peer X)
//! - Periodic sweep (timer; cadence per `custody_sweep_seconds`)
//!
//! Each trigger calls `reconcile_pass`, which is idempotent.
//!
//! For each `custody-blob` commitment in `rea_commitments`:
//! - If this peer is the provider AND blob_cid is missing locally:
//!   query `peer_blob_inventory` for hosts; if any candidates exist,
//!   kick a fetch via `blob_fetch::race_fetch` (T17). When the inventory
//!   is EMPTY, fall back to racing the currently-connected peers (capped
//!   at [`FALLBACK_CANDIDATE_CAP`]) — gossip inventory publish dies on
//!   every pod restart (`InsufficientPeers`), so an empty inventory is
//!   NOT evidence the mesh lacks the blob. Mirrors `get_blob_or_heal`'s
//!   connected-fallback, which is what makes the CI cross-pod fetch pass.
//! - If this peer is the receiver AND last_seen for the provider is older
//!   than `placement_grace_seconds`: emit a `placement-gap` REA event
//!   (with cooldown).
//!
//! Local store presence is queried via the trait `LocalBlobStore` so the
//! reconcile pass can be unit-tested without a real blob store.

use crate::blob_store::{BlobStore, Exists};
use crate::db::diesel_schema::{economic_events, peer_blob_inventory, rea_commitments};
use crate::db::models::{NewEconomicEvent, ReaCommitment};
use crate::error::StorageError;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use elohim_epr::address::ByteCid;

use super::placement::{PlacementCandidate, PlacementStrategy};

/// Trait for checking whether a blob exists in this peer's local store.
/// Production: queries the blob_store. Tests: returns a fixed set.
pub trait LocalBlobStore: Send + Sync {
    fn has(&self, blob_cid: &ByteCid) -> bool;
}

/// Trait for kicking a fetch. Production: calls `blob_fetch::race_fetch` (T17).
/// Tests: records the kick request.
pub trait FetchKicker: Send + Sync {
    fn kick(&self, blob_cid: &ByteCid, candidates: Vec<String>);
}

/// Snapshot of the local blob store's payload NAMES taken at reconcile-pass start.
///
/// Implements [`LocalBlobStore`] for the duration of a single pass by
/// pre-fetching the full CID set once via [`crate::blob_store::BlobStore::list_cids`]
/// and answering `has` from a [`std::collections::HashSet`]. This is cheaper
/// (and avoids holding the blob store across diesel queries) than calling
/// `BlobStore::exists` per commitment. Comparison is by [`ByteCid`] value (C4):
/// a commitment marker is parsed by full decode before it is compared.
///
/// It is a filename set, not a verification (see [`VerifiedPantry`] for the verified one): a payload whose marker is present but
/// whose bytes are corrupt or whose chunks are missing is still listed, and chunk or
/// shard units are not listed at all. It answers "is a fetch needed?" (a wrong yes
/// costs a skipped fetch, healed by verify-on-read). It must never gate a custody
/// row, a pledge or an announcement — those rest on a verified read
/// (`BlobStore::get` / `get_chunk`, or `exists` == `Verified`).
pub struct BlobStoreSnapshot {
    cids: std::collections::HashSet<ByteCid>,
}

impl BlobStoreSnapshot {
    /// Build a snapshot by reading the local pantry once. Returns
    /// [`StorageError`] if the directory walk fails; the caller should skip
    /// the reconcile tick rather than running with a partial view.
    pub fn from_store(store: &crate::blob_store::BlobStore) -> Result<Self, StorageError> {
        let cids = store.list_cids()?.into_iter().collect();
        Ok(Self { cids })
    }

    /// Number of CIDs captured in the snapshot. Useful for tests + tracing.
    pub fn len(&self) -> usize {
        self.cids.len()
    }

    /// `true` when the snapshot captured no CIDs.
    pub fn is_empty(&self) -> bool {
        self.cids.is_empty()
    }
}

impl LocalBlobStore for BlobStoreSnapshot {
    fn has(&self, blob_cid: &ByteCid) -> bool {
        self.cids.contains(blob_cid)
    }
}

/// A [`LocalBlobStore`] that answers `has` from a verified read, per query: the payload's
/// bytes (or the unit's) hash to their CID, or the answer is no. Use it wherever `has`
/// gates a custody act or a "do I still need to fetch this?" decision; a corrupt local
/// copy is "not held" (and quarantined), so it is re-fetched rather than pledged or
/// advertised. Cost is the bytes of each queried CID, so it suits the few CIDs a pass
/// asks about, not a whole-store scan.
pub struct VerifiedPantry<'a>(&'a crate::blob_store::BlobStore);

impl<'a> VerifiedPantry<'a> {
    pub fn new(store: &'a crate::blob_store::BlobStore) -> Self {
        Self(store)
    }
}

impl LocalBlobStore for VerifiedPantry<'_> {
    fn has(&self, blob_cid: &ByteCid) -> bool {
        self.0.holds_verified(blob_cid)
    }
}

/// Cap on connected peers raced by an inventory-blind fallback kick.
/// Mirrors the `get_blob_or_heal` connected-fallback cap in `http.rs`.
pub const FALLBACK_CANDIDATE_CAP: usize = 8;

/// Outcome of a single reconcile pass; useful for tests + metrics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReconcileOutcome {
    pub kicks_fired: u32,
    /// Subset of `kicks_fired` that raced connected peers because the
    /// gossip inventory had no candidates (inventory-blind fallback).
    pub fallback_kicks: u32,
    pub placement_gaps_emitted: u32,
    pub commitments_examined: u32,
    /// Commitments whose blob marker is not a valid content address in ANY
    /// byte CID (`ByteCid::parse` refused it).
    /// These are skipped WITHOUT a fetch kick: no responder can accept the
    /// address, so retrying every sweep tick would drum the wire forever
    /// (the T21 rejection drumbeat). The count is the honest failure record
    /// — a non-zero value means rows need healing, not more retries.
    pub invalid_markers: u32,
}

/// Timing parameters for a reconcile pass.
#[derive(Debug, Clone, Copy)]
pub struct ReconcileConfig {
    /// How long a custody commitment can be unhonored before placement-gap fires.
    pub placement_grace_seconds: u64,
    /// Minimum time between repeated placement-gap events for the same commitment.
    pub placement_gap_cooldown_seconds: u64,
    /// TTL for peer_blob_inventory entries before they're considered stale.
    pub inventory_freshness_seconds: u64,
}

/// Run one reconcile pass over the custody-blob commitments visible in this
/// peer's projection. Idempotent.
/// `self_cid` is this node's TRANSPORT id (`Config::self_cid`); `self_agent_cid`
/// is its resolved holochain `agent_cid` (`uhCAk…`), when available. Self-matching
/// (own-provider → fetch-on-miss; own-receiver → placement-gap) matches a
/// commitment row whose `provider`/`receiver` equals EITHER namespace. This is
/// the transition semantics: legacy rows (and pre-resolver salvage writes) carry
/// the transport id, while new salvage/seeder writes carry the agent_cid — so
/// both must be recognised or the node's own rows are orphaned (the fetch never
/// fires for an agent_cid-provider row this node authored). `self_agent_cid` is
/// `None` when the conductor cell key was not resolvable at P2P boot; matching
/// Resolve THIS node's `agent_cid` for reconcile self-matching, IN LOCKSTEP with
/// the salvage author's per-tick resolution
/// (`services::salvage_commitment_author::resolve_self_agent_cid`): the active
/// session's `agent_pub_key` re-resolved from `conn` on EVERY pass, falling back
/// to the boot-time conductor cell-key snapshot. Re-resolving the session arm per
/// pass is what keeps the two in lockstep — otherwise a session that logs in (or
/// changes) AFTER P2P boot would make salvage author `provider = <session
/// agent_cid>` rows that a frozen reconcile value could never match (the orphaned
/// own-row class). Returns `None` only when neither candidate is an `agent_cid`;
/// matching then degrades to transport-only (the pre-existing behavior).
pub fn resolve_self_agent_cid(
    conn: &mut SqliteConnection,
    cellkey_snapshot: Option<&str>,
) -> Option<String> {
    let session_cid = crate::db::local_sessions::get_active_session(conn)
        .ok()
        .flatten()
        .map(|s| s.agent_pub_key);
    crate::identity_namespace::resolve_agent_cid_write(&[session_cid.as_deref(), cellkey_snapshot])
}

/// then degrades to transport-only (the pre-existing behavior — no regression).
#[allow(
    clippy::too_many_arguments,
    reason = "each argument is an independent seam (two self identities, the local store, the fetch kicker, timing config, clock, connected peers) that tests substitute; a bundling struct would only rename them"
)]
pub fn reconcile_pass(
    conn: &mut SqliteConnection,
    self_cid: &str,
    self_agent_cid: Option<&str>,
    local_store: &dyn LocalBlobStore,
    fetch_kicker: &dyn FetchKicker,
    cfg: ReconcileConfig,
    now: DateTime<Utc>,
    connected_peers: &[String],
) -> Result<ReconcileOutcome, StorageError> {
    // Does `s` name THIS node in either identity namespace (transport OR agent)?
    let is_self = |s: &str| s == self_cid || self_agent_cid == Some(s);
    let placement_grace_seconds = cfg.placement_grace_seconds;
    let placement_gap_cooldown_seconds = cfg.placement_gap_cooldown_seconds;
    let inventory_freshness_seconds = cfg.inventory_freshness_seconds;
    let mut outcome = ReconcileOutcome::default();

    // Only LIVE obligations drive a pass. A cancelled, terminated or
    // superseded row records what was once promised; it asks nothing of this
    // node now. Sweeping them was half of matthew's 2026-09-13 saturation —
    // five rows named the same 231 MB blob, one of them cancelled, and every
    // pass re-raced a commitment nobody held.
    let custody_rows = rea_commitments::table
        .filter(rea_commitments::action.eq("custody-blob"))
        .filter(rea_commitments::state.ne_all(crate::db::models::commitment_withdrawn_states::ALL))
        .load::<ReaCommitment>(conn)
        .map_err(|e| StorageError::Database(format!("load custody-blob commitments: {e}")))?;

    let stale_before = (now - chrono::Duration::seconds(inventory_freshness_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let unhonored_before = (now - chrono::Duration::seconds(placement_grace_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let cooldown_after = (now - chrono::Duration::seconds(placement_gap_cooldown_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    for commitment in custody_rows {
        outcome.commitments_examined += 1;
        // `resource_classified_as` is a JSON list by contract (non-commons spec
        // §11.2, Option A); a custody-blob row carries a single blob hash as the
        // list's primary element. Reading the raw column directly was
        // latent-buggy on array-wrapped `["sha256-…"]` rows (U1, 2026-06-19) —
        // route through the accessor so both bare and list forms resolve.
        let Some(raw_marker) = commitment.primary_classification() else {
            continue;
        };

        // Address hygiene BEFORE any fetch decision: a marker is a byte CID by
        // full decode or it is nothing. A `sha256-…`, bare hex, double-wrapped or
        // sha2 CID marker cannot name bytes this plane serves, so it is counted
        // and skipped (the row needs re-minting, retries cannot succeed) — never
        // kicked, never re-keyed.
        let blob_cid = match ByteCid::parse(&raw_marker) {
            Ok(cid) => cid,
            Err(error) => {
                outcome.invalid_markers += 1;
                tracing::warn!(
                    target: "elohim_storage::reconcile",
                    commitment_id = %commitment.id,
                    marker = %raw_marker,
                    check = error.check(),
                    "custody-blob marker is not a byte CID; skipping (no fetch kick)"
                );
                continue;
            }
        };
        let blob_key = blob_cid.to_string();

        if is_self(&commitment.provider) {
            // Own commitment — act if missing.
            if !local_store.has(&blob_cid) {
                let candidates =
                    crate::db::peer_blob_inventory::lookup_hosts(conn, &blob_key, &stale_before)?;
                if !candidates.is_empty() {
                    let inventory: Vec<String> =
                        candidates.into_iter().map(|c| c.peer_id).collect();

                    // Wave-3: re-order inventory by capability/headroom/RTT/bond/diversity.
                    // `select_serve_peers` → agent_cids → resolved to libp2p via manifest.
                    // Empty manifest (prod default) → prefix empty → inventory unchanged.
                    // Resolution requires peer_transport_manifest (iroh feature).
                    // Without it the prefix is empty → inventory unchanged (safe degradation).
                    #[cfg(feature = "p2p-iroh")]
                    let score_prefix: Vec<String> =
                        crate::services::serve_routing::select_serve_peers(
                            conn,
                            &blob_key,
                            inventory.len().max(8),
                        )
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|cid| {
                            crate::p2p_iroh::peer_map::lookup_by_agent_cid(conn, &cid)
                                .ok()
                                .flatten()
                                .and_then(|m| m.libp2p.map(|l| l.peer_id))
                        })
                        .collect();
                    #[cfg(not(feature = "p2p-iroh"))]
                    let score_prefix: Vec<String> = Vec::new();

                    let candidate_peers: Vec<String> = if score_prefix.is_empty() {
                        inventory
                    } else {
                        let prefix_set: std::collections::HashSet<&str> =
                            score_prefix.iter().map(String::as_str).collect();
                        let tail: Vec<String> = inventory
                            .into_iter()
                            .filter(|p| !prefix_set.contains(p.as_str()))
                            .collect();
                        let mut merged = score_prefix;
                        merged.extend(tail);
                        merged
                    };

                    fetch_kicker.kick(&blob_cid, candidate_peers);
                    outcome.kicks_fired += 1;
                } else if !connected_peers.is_empty() {
                    // Inventory-blind fallback: gossip inventory publish dies
                    // on every pod restart (InsufficientPeers), so an empty
                    // candidate set is NOT evidence the mesh lacks the blob.
                    // Race the connected peers, capped — the same fallback
                    // that makes heal-on-read pass the CI cross-pod fetch.
                    let fallback: Vec<String> = connected_peers
                        .iter()
                        .take(FALLBACK_CANDIDATE_CAP)
                        .cloned()
                        .collect();
                    fetch_kicker.kick(&blob_cid, fallback);
                    outcome.kicks_fired += 1;
                    outcome.fallback_kicks += 1;
                }
            }
        } else if is_self(&commitment.receiver) {
            // Other peer's commitment to me — observe; signal on stale.
            let last_seen: Option<String> = peer_blob_inventory::table
                .filter(peer_blob_inventory::peer_id.eq(&commitment.provider))
                .filter(peer_blob_inventory::blob_cid.eq(&blob_key))
                .select(peer_blob_inventory::last_seen_at)
                .first::<String>(conn)
                .optional()
                .map_err(|e| {
                    StorageError::Database(format!("lookup last_seen for placement-gap: {e}"))
                })?;

            let unhonored = match last_seen {
                None => true, // never observed
                Some(ts) => ts.as_str() < unhonored_before.as_str(),
            };

            if !unhonored {
                continue;
            }

            // Cooldown: don't re-emit if a recent placement-gap event for the
            // same commitment exists.
            let recent_gap = economic_events::table
                .filter(economic_events::action.eq("placement-gap"))
                .filter(economic_events::output_of.eq(&commitment.id))
                .filter(economic_events::has_point_in_time.gt(&cooldown_after))
                .count()
                .get_result::<i64>(conn)
                .map_err(|e| StorageError::Database(format!("count recent placement-gap: {e}")))?;

            if recent_gap > 0 {
                continue;
            }

            // Emit placement-gap event.
            // Bind owned strings so we can borrow them into NewEconomicEvent<'_>.
            let event_id = uuid::Uuid::new_v4().to_string();
            let has_point = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let new_event = NewEconomicEvent {
                id: &event_id,
                h_app_id: &commitment.h_app_id,
                action: "placement-gap",
                provider: &commitment.provider,
                receiver: &commitment.receiver,
                resource_conforms_to: commitment.resource_conforms_to.as_deref(),
                resource_inventoried_as: Some(blob_key.as_str()),
                resource_classified_as_json: None,
                resource_quantity_value: None,
                resource_quantity_unit: None,
                effort_quantity_value: None,
                effort_quantity_unit: None,
                has_point_in_time: &has_point,
                has_duration: None,
                input_of: None,
                output_of: Some(commitment.id.as_str()),
                lamad_event_type: None,
                content_id: None,
                contributor_presence_id: None,
                path_id: None,
                triggered_by: None,
                state: "observed",
                note: None,
                metadata_json: None,
                dht_anchor_hash: None,
                at_location: None,
                verified_at: None,
                scope_collab_cid: None,
                bounded_by: None,
                substrate_signal: None,
            };
            diesel::insert_into(economic_events::table)
                .values(&new_event)
                .execute(conn)
                .map_err(|e| StorageError::Database(format!("insert placement-gap event: {e}")))?;
            outcome.placement_gaps_emitted += 1;
        }
    }

    Ok(outcome)
}

/// Whether this node offers salvage custody, and the target replication level.
///
/// `enabled` is the opt-in consent gate (default false at the config layer; a
/// node is NEVER conscripted — the imago-dei floor). `target_replicas` is the
/// desired distinct-holder count per blob; `inventory_freshness_seconds` bounds
/// what counts as "currently hosting" when computing honored replicas.
#[derive(Debug, Clone, Copy)]
pub struct SalvageConfig {
    pub enabled: bool,
    pub target_replicas: usize,
    pub inventory_freshness_seconds: u64,
}

/// Authors a salvage `custody-blob` commitment naming `provider` as the new
/// holder of `blob_marker`, owed to content steward `receiver`.
///
/// Production (P3-5) mints a NOTARIZED Mishpat commitment via the conductor so
/// the placement intent stays Class-A and projects back into `rea_commitments`.
/// Tests record the request. Implementors must be idempotent-friendly — the
/// caller already skips when self is a provider, but a duplicate author must not
/// corrupt state.
pub trait CommitmentAuthor: Send + Sync {
    fn author_custody_blob(
        &self,
        blob_marker: &str,
        provider: &str,
        receiver: &str,
    ) -> Result<(), StorageError>;
}

/// Outcome of a single salvage pass; useful for tests + metrics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SalvageOutcome {
    pub blobs_examined: u32,
    pub under_replicated: u32,
    pub commitments_authored: u32,
    pub skipped_not_closest: u32,
    pub skipped_opted_out: u32,
}

/// The spellings one holder's custody evidence may be keyed by: the provider itself
/// (a legacy commitment naming a transport id directly) and every transport id the
/// cross-signed chain authenticates to it (`p2p::transport_authority`: the libp2p
/// PeerIds its `AgentPeerBinding`s cross-sign, and the iroh NodeIds verified two-key
/// links tie to those). Never `peer_transport_manifest`: its `agent_cid` is a
/// self-asserted dial hint, and a forged manifest must not inflate or deflate a floor.
fn holder_identities(
    conn: &mut SqliteConnection,
    provider: &str,
) -> Result<Vec<String>, StorageError> {
    let mut identities = vec![provider.to_string()];
    for id in crate::p2p::transport_authority::authenticated_transports_for_agent(conn, provider) {
        if !identities.contains(&id) {
            identities.push(id);
        }
    }
    Ok(identities)
}

/// Run one salvage pass (Phase 3). For each under-replicated `custody-blob`
/// blob, if this node is opted in, not already a provider, and among the
/// deterministic closest-N holders for the blob (per `strategy`), author a new
/// custody-blob commitment naming self as provider. The existing
/// [`reconcile_pass`] provider branch then fetches the bytes — **salvage authors
/// intent; reconcile moves bytes** (no new fetch path).
///
/// Coordination-free: every peer computes the same closest-N over the same pool,
/// so exactly the intended holders self-select (no thundering herd). Additive —
/// does NOT change [`reconcile_pass`]. Idempotent. The XOR vs intentional choice
/// lives behind `strategy` (the [`PlacementStrategy`] seam) — see
/// `2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md`.
/// ## Self-identity (two namespaces)
///
/// `self_read_cid` is the node's TRANSPORT id (`Config::self_cid`) and matches
/// only existing rows this node authored BEFORE the agent_cid resolver landed.
/// `self_write_cid` is the resolved holochain `agent_cid` (`uhCAk…`) — the
/// candidate-pool identity, the placement rank-match, and the `provider` this
/// pass AUTHORS. The "already a provider" idempotency check matches EITHER (so a
/// legacy transport-id row and a new agent_cid row both count), but every WRITE
/// uses `self_write_cid`. `run_salvage_pass` guarantees `self_write_cid` is a
/// valid agent_cid (it skips the tick otherwise).
#[allow(
    clippy::too_many_arguments,
    reason = "each argument is an independent seam (two identities, strategy, author, pool, config, clock) the tests substitute; a bundling struct would only rename them"
)]
pub fn salvage_pass(
    conn: &mut SqliteConnection,
    self_read_cid: &str,
    self_write_cid: &str,
    strategy: &dyn PlacementStrategy,
    author: &dyn CommitmentAuthor,
    salvage_pool: &[PlacementCandidate],
    cfg: SalvageConfig,
    now: DateTime<Utc>,
) -> Result<SalvageOutcome, StorageError> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut outcome = SalvageOutcome::default();

    // Only LIVE obligations drive a pass. A cancelled, terminated or
    // superseded row records what was once promised; it asks nothing of this
    // node now. Sweeping them was half of matthew's 2026-09-13 saturation —
    // five rows named the same 231 MB blob, one of them cancelled, and every
    // pass re-raced a commitment nobody held.
    let custody_rows = rea_commitments::table
        .filter(rea_commitments::action.eq("custody-blob"))
        .filter(rea_commitments::state.ne_all(crate::db::models::commitment_withdrawn_states::ALL))
        .load::<ReaCommitment>(conn)
        .map_err(|e| StorageError::Database(format!("load custody-blob commitments: {e}")))?;

    let fresh_before = (now - chrono::Duration::seconds(cfg.inventory_freshness_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    // Group commitments by blob marker → (distinct providers, a content steward).
    // BTree* keeps the pass deterministic (stable blob iteration order).
    struct BlobWant {
        providers: BTreeSet<String>,
        receiver: String,
    }
    let mut wants: BTreeMap<String, BlobWant> = BTreeMap::new();
    for c in &custody_rows {
        // Only a byte-CID marker names bytes; anything else is skipped here as it
        // is in `reconcile_pass`.
        let Some(blob) = c
            .primary_classification()
            .and_then(|m| ByteCid::parse(&m).ok())
            .map(|cid| cid.to_string())
        else {
            continue;
        };
        let entry = wants.entry(blob).or_insert_with(|| BlobWant {
            providers: BTreeSet::new(),
            receiver: c.receiver.clone(),
        });
        entry.providers.insert(c.provider.clone());
    }

    for (blob, want) in wants {
        outcome.blobs_examined += 1;

        // honored = distinct committed providers with BOUND, unexpired custody
        // evidence. A row counts only when its witness bound the peer id to the
        // speaker (signed author, end-to-end signature, recon over the peer's own
        // connection, a verified fetch, this node's own verified store); a
        // `legacy-hint` row stays a fetch hint and an expired row is no evidence. A
        // HELD envelope (unknown epoch or scheme) is never projected at all.
        //
        // Inventory rows are keyed by the author's TRANSPORT id while a provider is
        // an `agent_cid` (or, for legacy rows, a transport id), so a provider's
        // evidence is read under every spelling `peer_transport_manifest` binds to it.
        let mut honored = 0usize;
        for provider in &want.providers {
            let identities = holder_identities(conn, provider)?;
            if crate::db::peer_blob_inventory::holds_bound_fresh(
                conn,
                &identities,
                &blob,
                &fresh_before,
            )? {
                honored += 1;
            }
        }

        if honored >= cfg.target_replicas {
            continue; // already resilient
        }
        outcome.under_replicated += 1;

        if !cfg.enabled {
            outcome.skipped_opted_out += 1;
            continue; // opt-in consent gate (imago-dei floor; a node is never conscripted)
        }
        // "Already a provider" is checked in BOTH namespaces: a row this node
        // authored last tick carries `self_write_cid` (agent_cid), while a legacy
        // row it authored before the resolver carries `self_read_cid` (transport).
        // Either means "no duplicate adopt".
        if want.providers.contains(self_write_cid) || want.providers.contains(self_read_cid) {
            continue; // already a provider for this blob — idempotent, no duplicate adopt
        }

        // Deterministic self-selection: adopt iff self is among the closest-N
        // holders for this blob. The pool identifies self by its agent_cid, so the
        // match is against `self_write_cid`. Every peer computes the same set over
        // the same pool, so exactly the intended holders adopt (coordination-free).
        let closest = strategy.rank(&blob, salvage_pool, cfg.target_replicas);
        if !closest.iter().any(|cid| cid == self_write_cid) {
            outcome.skipped_not_closest += 1;
            continue;
        }

        // Adopt: author the notarized placement intent naming self by agent_cid
        // (NEVER a transport id); the next reconcile_pass provider branch fetches
        // the bytes.
        author.author_custody_blob(&blob, self_write_cid, &want.receiver)?;
        outcome.commitments_authored += 1;
    }

    Ok(outcome)
}

// ────────────────────────────────────────────────────────────────────────
// Custody manifest + self-held evidence backfill.
//
// `services::shard_manifest_backfill` heals CONTENT rows (walks the `content`
// table for a `blob_cid` with no matching `shard_manifests` row). That leaves
// a gap: blobs stored through the raw blob-plane ingest path (`PUT /blob/{hash}`
// → `http.rs::put_blob_bytes`) that predate the manifest producer (commit
// 123fd4bd5) have NO content row to anchor on at all, so the content-table scan
// can never see them. This pass instead enumerates the BLOB STORE directly
// (`BlobStore::list_cids`) — the same surface the inventory broadcaster and
// `BlobStoreSnapshot` already use — so it also catches those content-less
// legacy blobs.
//
// For each blob missing a `shard_manifests` row it derives + persists a
// manifest the EXACT way `put_blob_bytes` does (same `ShardEncoder`, same
// `record_generated_manifest` call, same `blob:{cid}` projection key), then —
// new — records self-held evidence for the shard(s) this node just verified it
// holds (`services::self_stewardship::record_self_held_shard`). This flips the
// custody fold's `unknown` class to a real class (`stocked`, when an active
// custody-blob commitment names the holder) WITHOUT a fresh PUT or a
// `distribute_shards` round.
//
// Reconciliation, not conscription: this is bytes-we-already-hold, so it is
// NEVER gated behind `salvage_capacity_enabled` (that flag governs volunteering
// for NEW custody). It has its own config flag (`Config::manifest_backfill_enabled`,
// default ON) wired in `main.rs`.
// ────────────────────────────────────────────────────────────────────────

/// Per-pass cap on blobs actually ENCODED + PERSISTED (the missing-manifest
/// candidate set), not on the raw scan. The candidacy pre-filter
/// (`cid → existing-manifest?`) is a cheap DB-only check with no byte
/// I/O, so it walks every CID the store reports; only the genuine gap is
/// capped. This mirrors `shard_manifest_backfill::REDISTRIBUTE_CAP_PER_BOOT`'s
/// reasoning: a huge already-manifested pantry can never starve later hashes
/// across ticks, because the cap only ever bites the real candidates.
pub const MANIFEST_BACKFILL_BATCH_CAP: usize = 256;

/// Outcome counters for one [`manifest_backfill_pass`] run; logged verbatim by
/// the caller and useful for tests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ManifestBackfillOutcome {
    /// Total blob hashes the local store reported this pass.
    pub blobs_scanned: usize,
    /// New `shard_manifests` rows persisted.
    pub manifests_written: usize,
    /// New `shard_locations` self-held rows recorded.
    pub self_held_recorded: usize,
    /// Self-held rows RE-claimed for blobs that already had a manifest — the
    /// re-key repair arm (see `manifest_backfill_pass`). Counted separately from
    /// `self_held_recorded` so "the node re-attributed old custody to its live
    /// key" is distinguishable in logs from "the node manifested new bytes".
    pub self_held_reclaimed: usize,
    /// Self-held rows recorded by the unit sweep: a custody unit (a chunk file, an inline
    /// payload or a received shard) the store read back verified that no row named yet.
    /// This is how a striped upload's chunks and a received shard get a row.
    pub units_recorded: usize,
    /// Candidates that turned out to already have a manifest (pre-filter hit,
    /// or a race detected at the write-time re-check).
    pub skipped_existing: usize,
    /// Lookup/read/encode/persist failures, or a local-bytes hash-verification
    /// failure (never a fatal abort — logged and the pass continues).
    pub errors: usize,
}

/// [`LocalBlobStore`] naming exactly the units this pass just verified: the chunks of
/// a payload whose whole BLAKE3 the store has recomputed. Avoids a second listing.
struct VerifiedUnits<'a>(&'a [ByteCid]);

impl LocalBlobStore for VerifiedUnits<'_> {
    fn has(&self, cid: &ByteCid) -> bool {
        self.0.contains(cid)
    }
}

/// The custody units of a persisted manifest row this node holds whole: the chunks of
/// an unstriped payload (a striped payload's shards are derived bytes the local store
/// does not hold under their own CIDs, so they are never self-claimed here).
fn unstriped_units(row: &crate::db::models::ShardManifestRow) -> Option<Vec<ByteCid>> {
    let manifest = crate::db::shard_manifests::hydrate_manifest(row).ok()?;
    manifest
        .stripes
        .is_empty()
        .then(|| manifest.custody_units())
}

/// Record self-held evidence for each unit not already claimed under `agent_cid`.
/// Returns how many rows were recorded; failures are counted into `errors`.
fn claim_units(
    conn: &mut SqliteConnection,
    units: &[ByteCid],
    agent_cid: &str,
    h_app_id: &str,
    errors: &mut usize,
) -> usize {
    let probe = VerifiedUnits(units);
    let mut recorded = 0;
    for unit in units {
        let already = crate::db::shard_locations::get_locations_for_shard(conn, &unit.to_string())
            .map(|rows| rows.iter().any(|l| l.peer_id == agent_cid))
            .unwrap_or(false);
        if already {
            continue;
        }
        match crate::services::self_stewardship::record_self_held_shard(
            conn, &probe, unit, agent_cid, h_app_id,
        ) {
            Ok(crate::services::self_stewardship::SelfHeldOutcome::Recorded) => recorded += 1,
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::reconcile",
                    unit = %unit,
                    error = %e,
                    "custody-backfill: self-held recording failed"
                );
                *errors += 1;
            }
        }
    }
    recorded
}

/// Row every custody unit the store holds that no self-held row names yet. The candidates
/// are `BlobStore::unit_names` (a filename set, never evidence); each is read back through
/// `get_chunk`, which re-derives its CID and quarantines a failure, and only a verified
/// unit is recorded. This reaches what the per-payload pass cannot: the chunks of a payload
/// whose manifest is striped (its shards are derived, but the node serves its chunks) and
/// a shard another node pushed here. At most `cap` rows are recorded per pass.
async fn claim_held_units(
    conn: &mut SqliteConnection,
    blob_store: &BlobStore,
    agent_cid: &str,
    h_app_id: &str,
    cap: usize,
    errors: &mut usize,
) -> usize {
    let mut recorded = 0;
    for unit in blob_store.unit_names() {
        if recorded >= cap {
            break;
        }
        let already = crate::db::shard_locations::get_locations_for_shard(conn, &unit.to_string())
            .map(|rows| rows.iter().any(|l| l.peer_id == agent_cid))
            .unwrap_or(false);
        if already || blob_store.get_chunk(&unit).await.is_err() {
            continue;
        }
        recorded += claim_units(conn, &[unit], agent_cid, h_app_id, errors);
    }
    recorded
}

/// Run one bounded custody-backfill pass. Idempotent — safe to call on every tick.
///
/// For every payload the local store lists with no `shard_manifests` row under
/// `blob:{cid}`, read it (the store recomputes the whole BLAKE3 before returning, and
/// quarantines on mismatch — unverified bytes are never manifested), mint the manifest
/// the way the ingest path does (`ShardEncoder::encode`), persist it, and record
/// self-held evidence for its custody units when it is unstriped.
///
/// RE-CLAIM ARM: a payload that already has a manifest is still a self-held
/// candidate. This node's `agent_cid` changes across a non-prod DNA reinstall, and a
/// manifest written by the ingest path records no self-held row, so the claim is
/// re-asserted under the live identity (`self_held_reclaimed`) when the store still
/// holds the payload hash-verified ([`Exists::Verified`]). Steady state writes
/// nothing.
///
/// `self_agent_cid` is this node's resolved `agent_cid` (`uhCAk…`); `None` skips
/// self-held recording for the pass. `record_self_held_shard` refuses anything that is
/// not a well-formed `agent_cid`.
pub async fn manifest_backfill_pass(
    conn: &mut SqliteConnection,
    blob_store: &BlobStore,
    self_agent_cid: Option<&str>,
    h_app_id: &str,
    batch_cap: usize,
) -> Result<ManifestBackfillOutcome, StorageError> {
    let mut outcome = ManifestBackfillOutcome::default();
    let cids = blob_store.list_cids()?;
    outcome.blobs_scanned = cids.len();
    let agent = self_agent_cid.filter(|c| crate::identity_namespace::is_agent_cid(c));

    let mut candidates: Vec<(ByteCid, String)> = Vec::new();
    // Re-claim work is capped independently so a store full of already-manifested
    // payloads cannot make one pass unbounded.
    let mut reclaims: usize = 0;
    for cid in &cids {
        if candidates.len() >= batch_cap {
            break;
        }
        let content_id = format!("blob:{cid}");
        match crate::db::shard_manifests::get_manifest(conn, h_app_id, &content_id) {
            Ok(Some(existing)) => {
                outcome.skipped_existing += 1;
                let Some(agent_cid) = agent else {
                    continue;
                };
                if reclaims >= batch_cap {
                    continue;
                }
                let Some(units) = unstriped_units(&existing) else {
                    continue;
                };
                if blob_store.exists(cid).await != Exists::Verified {
                    continue;
                }
                reclaims += 1;
                outcome.self_held_reclaimed +=
                    claim_units(conn, &units, agent_cid, h_app_id, &mut outcome.errors);
            }
            Ok(None) => candidates.push((*cid, content_id)),
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::reconcile",
                    cid = %cid,
                    error = %e,
                    "custody-backfill: manifest lookup failed; skipping"
                );
                outcome.errors += 1;
            }
        }
    }

    let encoder = crate::sharding::ShardEncoder::new(crate::sharding::ShardConfig::default());

    for (cid, content_id) in candidates {
        // `get` recomputes the whole BLAKE3 and refuses (quarantining) on mismatch.
        let data = match blob_store.get(&cid).await {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::reconcile",
                    cid = %cid,
                    error = %e,
                    "custody-backfill: payload unreadable or failed verification; skipping"
                );
                outcome.errors += 1;
                continue;
            }
        };

        // Same derivation as the ingest path; the mime type is unknown for a raw
        // enumerate and does not affect chunking or erasure.
        let manifest = match encoder.create_manifest(&data, "application/octet-stream", "commons") {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::reconcile",
                    cid = %cid,
                    error = %e,
                    "custody-backfill: manifest encoding failed; skipping"
                );
                outcome.errors += 1;
                continue;
            }
        };

        // Defensive re-check: a concurrent writer may have raced a manifest in.
        match crate::db::shard_manifests::get_manifest(conn, h_app_id, &content_id) {
            Ok(Some(_)) => {
                outcome.skipped_existing += 1;
                continue;
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::reconcile",
                    cid = %cid,
                    error = %e,
                    "custody-backfill: manifest re-check failed; skipping"
                );
                outcome.errors += 1;
                continue;
            }
        }

        if let Err(e) = crate::db::shard_manifests::record_generated_manifest(
            conn,
            &content_id,
            h_app_id,
            &manifest,
        ) {
            tracing::warn!(
                target: "elohim_storage::reconcile",
                cid = %cid,
                error = %e,
                "custody-backfill: manifest persist failed"
            );
            outcome.errors += 1;
            continue;
        }
        outcome.manifests_written += 1;

        if let (Some(agent_cid), true) = (agent, manifest.stripes.is_empty()) {
            let units = manifest.custody_units();
            outcome.self_held_recorded +=
                claim_units(conn, &units, agent_cid, h_app_id, &mut outcome.errors);
        }
    }

    if let Some(agent_cid) = agent {
        outcome.units_recorded += claim_held_units(
            conn,
            blob_store,
            agent_cid,
            h_app_id,
            batch_cap,
            &mut outcome.errors,
        )
        .await;
    }

    tracing::info!(
        target: "elohim_storage::reconcile",
        blobs_scanned = outcome.blobs_scanned,
        manifests_written = outcome.manifests_written,
        units_recorded = outcome.units_recorded,
        self_held_recorded = outcome.self_held_recorded,
        self_held_reclaimed = outcome.self_held_reclaimed,
        skipped_existing = outcome.skipped_existing,
        errors = outcome.errors,
        "custody-backfill: pass complete"
    );

    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::diesel_schema::rea_commitments;
    use crate::db::models::NewReaCommitment;

    /// A byte-CID marker for a fixture label (the CID of the label's bytes).
    fn cid_of(label: char) -> String {
        elohim_epr::address::compute_bytes_cid(&[label as u8]).to_string()
    }

    /// The peer's inventory is exactly `cids`: a fresh peer's first delta (snapshots
    /// are gone; recon or a first delta establishes a peer's set).
    fn apply_snapshot(
        conn: &mut SqliteConnection,
        peer: &str,
        cids: &[String],
        sequence: i64,
        when: &str,
    ) -> Result<(), StorageError> {
        crate::db::peer_blob_inventory::apply_delta(
            conn,
            peer,
            cids,
            &[],
            sequence,
            when,
            crate::db::peer_blob_inventory::InventoryWitness::SignedAuthor,
        )
        .map(|_| ())
    }
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};
    use std::sync::Mutex;

    fn test_pool() -> DbPool {
        let url = format!(
            "file:cust_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn default_cfg() -> ReconcileConfig {
        ReconcileConfig {
            placement_grace_seconds: 300,
            placement_gap_cooldown_seconds: 1800,
            inventory_freshness_seconds: 600,
        }
    }

    struct StaticStore(Vec<String>);
    impl LocalBlobStore for StaticStore {
        fn has(&self, cid: &ByteCid) -> bool {
            self.0.iter().any(|h| *h == cid.to_string())
        }
    }

    struct RecordingKicker {
        kicks: Mutex<Vec<(String, Vec<String>)>>,
    }
    impl FetchKicker for RecordingKicker {
        fn kick(&self, blob_cid: &ByteCid, candidates: Vec<String>) {
            self.kicks
                .lock()
                .unwrap()
                .push((blob_cid.to_string(), candidates));
        }
    }

    fn insert_custody_commitment(
        conn: &mut SqliteConnection,
        id: &str,
        provider: &str,
        receiver: &str,
        blob_cid: &str,
    ) {
        let row = NewReaCommitment {
            id,
            h_app_id: "test",
            action: "custody-blob",
            provider,
            receiver,
            resource_conforms_to: None,
            resource_classified_as: Some(blob_cid),
            resource_quantity_value: Some(1024.0),
            resource_quantity_unit: Some("bytes"),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: None,
            medium_of_exchange_id: None,
            state: "active",
            finished: 0,
            note: None,
            metadata_json: None,
            dht_anchor_hash: Some("hash1"),
        };
        diesel::insert_into(rea_commitments::table)
            .values(&row)
            .execute(conn)
            .unwrap();
    }

    #[test]
    fn own_commitment_with_local_blob_no_kick() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &cid_of('a'));

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![cid_of('a')]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &[],
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 0);
        assert!(kicker.kicks.lock().unwrap().is_empty());
    }

    #[test]
    fn own_commitment_with_missing_blob_kicks_when_candidate_exists() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_cid = cid_of('a');
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &blob_cid);

        // Use a fresh timestamp so the inventory entry is within freshness window.
        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_cid),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]), // local store empty
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 1);
        let kicks = kicker.kicks.lock().unwrap();
        assert_eq!(kicks.len(), 1);
        assert_eq!(kicks[0].0, blob_cid);
        assert_eq!(kicks[0].1, vec!["peer_X".to_string()]);
    }

    /// Insert a custody-blob commitment whose `resource_classified_as` is
    /// **array-wrapped** (`["<hash>"]`) — the JSON-list form the seeder/HTTP
    /// path writes (non-commons spec §11.2, Option A). Before the accessor fix
    /// this row read its raw `["<hash>"]` literal as the blob hash and never
    /// matched inventory.
    fn insert_custody_commitment_list(
        conn: &mut SqliteConnection,
        id: &str,
        provider: &str,
        receiver: &str,
        blob_cid: &str,
    ) {
        let classified = serde_json::to_string(&vec![blob_cid]).unwrap();
        let row = NewReaCommitment {
            id,
            h_app_id: "test",
            action: "custody-blob",
            provider,
            receiver,
            resource_conforms_to: None,
            resource_classified_as: Some(&classified),
            resource_quantity_value: Some(1024.0),
            resource_quantity_unit: Some("bytes"),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: None,
            medium_of_exchange_id: None,
            state: "active",
            finished: 0,
            note: None,
            metadata_json: None,
            dht_anchor_hash: Some("hash1"),
        };
        diesel::insert_into(rea_commitments::table)
            .values(&row)
            .execute(conn)
            .unwrap();
    }

    /// THE INTEGRATION TRAP: a custody-blob row stored array-wrapped
    /// (`["<hash>"]`) must still resolve to the real hash and fire the heal kick.
    /// The reader routes through `primary_classification()`, so the wrap is
    /// transparent. (RED before the accessor fix — the raw `["<hash>"]` string
    /// missed inventory and `kicks_fired == 0`.)
    #[test]
    fn own_commitment_array_wrapped_classified_kicks() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_cid = cid_of('b');
        insert_custody_commitment_list(&mut conn, "c1", "self_cid", "other_cid", &blob_cid);

        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_cid),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]), // local store empty
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        assert_eq!(
            outcome.kicks_fired, 1,
            "array-wrapped custody row must resolve to the real hash and kick"
        );
        let kicks = kicker.kicks.lock().unwrap();
        assert_eq!(kicks.len(), 1);
        assert_eq!(
            kicks[0].0, blob_cid,
            "the kicked hash is the unwrapped real blob hash, not the [..] literal"
        );
    }

    /// Bare-form regression: the pre-migration bare scalar still resolves
    /// (the accessor wraps it to a single-element list).
    #[test]
    fn own_commitment_bare_classified_still_kicks() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_cid = cid_of('c');
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &blob_cid);

        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_cid),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 1, "bare custody row still resolves");
        assert_eq!(kicker.kicks.lock().unwrap()[0].0, blob_cid);
    }

    /// Retry hygiene: a marker that is not a byte CID — a `sha256-<hex>`, bare hex,
    /// a CID double-wrapped in `sha256-`, a sha2 citation CID — is counted and
    /// skipped, never kicked and never re-keyed (a kick could only be refused by
    /// every responder on every sweep tick forever).
    #[test]
    fn legacy_spellings_are_invalid_markers_never_kicked() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let byte_cid = cid_of('z');
        let markers = [
            format!("sha256-{}", "a".repeat(64)),
            "a".repeat(64),
            format!("sha256-{byte_cid}"),
            elohim_epr::address::compute_raw_sha2_cid(b"z").to_string(),
            "sha256-not-hex-not-a-cid".to_string(),
        ];
        for (i, marker) in markers.iter().enumerate() {
            insert_custody_commitment(&mut conn, &format!("c{i}"), "self_cid", "other_cid", marker);
        }

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &["peer_A".to_string()], // connected fallback available — must still not kick
        )
        .unwrap();

        assert_eq!(outcome.invalid_markers, markers.len() as u32);
        assert_eq!(outcome.kicks_fired, 0);
        assert!(kicker.kicks.lock().unwrap().is_empty());
    }

    #[test]
    fn own_commitment_with_no_candidates_no_kick() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &cid_of('a'));

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &[],
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 0);
    }

    /// Inventory empty + connected peers present → the provider races the
    /// connected set (inventory-blind fallback). Gossip publish dies on every
    /// pod restart, so sweep convergence must not depend on inventory health.
    #[test]
    fn own_commitment_no_inventory_falls_back_to_connected_peers() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_cid = cid_of('a');
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &blob_cid);

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let connected = vec!["peer_A".to_string(), "peer_B".to_string()];
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &connected,
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 1);
        assert_eq!(outcome.fallback_kicks, 1);
        let kicks = kicker.kicks.lock().unwrap();
        assert_eq!(kicks.len(), 1);
        assert_eq!(kicks[0].0, blob_cid);
        assert_eq!(kicks[0].1, connected);
    }

    /// The fallback set is capped at FALLBACK_CANDIDATE_CAP connected peers
    /// (mirrors get_blob_or_heal's cap-8 race).
    #[test]
    fn fallback_caps_connected_candidates() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_cid = cid_of('a');
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &blob_cid);

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let connected: Vec<String> = (0..12).map(|i| format!("peer_{i}")).collect();
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &connected,
        )
        .unwrap();

        assert_eq!(outcome.fallback_kicks, 1);
        let kicks = kicker.kicks.lock().unwrap();
        assert_eq!(kicks[0].1.len(), FALLBACK_CANDIDATE_CAP);
        assert_eq!(kicks[0].1, connected[..FALLBACK_CANDIDATE_CAP].to_vec());
    }

    /// Fresh inventory candidates win over the fallback: the targeted kick
    /// uses the advertised host set, and fallback_kicks stays 0.
    #[test]
    fn inventory_candidates_preferred_over_connected_fallback() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_cid = cid_of('a');
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &blob_cid);

        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_cid),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now,
            &["peer_FALLBACK".to_string()],
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 1);
        assert_eq!(outcome.fallback_kicks, 0);
        let kicks = kicker.kicks.lock().unwrap();
        assert_eq!(kicks[0].1, vec!["peer_X".to_string()]);
    }

    #[test]
    fn others_commitment_unhonored_emits_placement_gap() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_cid = cid_of('a');
        insert_custody_commitment(&mut conn, "c1", "other_cid", "self_cid", &blob_cid);

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &[],
        )
        .unwrap();

        assert_eq!(outcome.placement_gaps_emitted, 1);
        assert_eq!(outcome.kicks_fired, 0);

        // Verify the event landed.
        use crate::db::diesel_schema::economic_events;
        let count: i64 = economic_events::table
            .filter(economic_events::action.eq("placement-gap"))
            .filter(economic_events::output_of.eq("c1"))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn others_commitment_with_fresh_inventory_no_gap() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_cid = cid_of('a');
        insert_custody_commitment(&mut conn, "c1", "other_cid", "self_cid", &blob_cid);

        // Fresh inventory entry from other_cid.
        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "other_cid",
            std::slice::from_ref(&blob_cid),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        assert_eq!(outcome.placement_gaps_emitted, 0);
    }

    #[test]
    fn placement_gap_cooldown_suppresses_repeat() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_cid = cid_of('a');
        insert_custody_commitment(&mut conn, "c1", "other_cid", "self_cid", &blob_cid);

        // First pass: emit gap.
        let now = chrono::Utc::now();
        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let _ = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        // Second pass within cooldown — should suppress.
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            None,
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now + chrono::Duration::seconds(60),
            &[],
        )
        .unwrap();

        assert_eq!(outcome.placement_gaps_emitted, 0);
    }

    // -----------------------------------------------------------------------
    // Identity backward-compat: reconcile matches self in EITHER namespace
    // -----------------------------------------------------------------------

    /// A NEW salvage-authored row carries `provider = agent_cid`. With the node's
    /// transport `self_cid` distinct from its `agent_cid`, the own-provider branch
    /// must still fire (kick on miss) by matching the agent_cid — otherwise the
    /// node's own salvage rows are orphaned and their bytes never fetched.
    #[test]
    fn reconcile_matches_own_agent_cid_provider_row() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_cid = cid_of('a');
        // provider = agent_cid (NOT the transport self_cid).
        insert_custody_commitment(&mut conn, "c1", "uhCAk-self", "steward-Z", &blob_cid);

        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_cid),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "12D3KooTransportSelfId", // transport self_cid
            Some("uhCAk-self"),       // resolved agent_cid
            &StaticStore(vec![]),     // blob missing locally
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        assert_eq!(
            outcome.kicks_fired, 1,
            "own agent_cid-provider row must match and kick"
        );
    }

    /// A LEGACY row carries `provider = transport self_cid` (authored before the
    /// resolver, or an env-pinned transport SELF_CID). Even with an agent_cid now
    /// resolved, the transport match must still fire — legacy rows are not orphaned.
    #[test]
    fn reconcile_still_matches_legacy_transport_provider_row() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_cid = cid_of('a');
        // provider = transport self_cid (legacy write).
        insert_custody_commitment(
            &mut conn,
            "c1",
            "12D3KooTransportSelfId",
            "steward-Z",
            &blob_cid,
        );

        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_cid),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "12D3KooTransportSelfId",
            Some("uhCAk-self"), // agent_cid resolved, but the row is legacy-transport
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now,
            &[],
        )
        .unwrap();

        assert_eq!(
            outcome.kicks_fired, 1,
            "legacy transport-id provider row must still match (not orphaned)"
        );
    }

    /// The receiver-side (placement-gap) branch also matches the agent_cid: a row
    /// whose `receiver = agent_cid` and provider is a stale peer emits a gap.
    #[test]
    fn reconcile_matches_own_agent_cid_receiver_row() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_cid = cid_of('a');
        insert_custody_commitment(&mut conn, "c1", "other_cid", "uhCAk-self", &blob_cid);

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "12D3KooTransportSelfId",
            Some("uhCAk-self"),
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
            &[],
        )
        .unwrap();

        assert_eq!(
            outcome.placement_gaps_emitted, 1,
            "own agent_cid-receiver row must emit a placement-gap"
        );
    }

    /// Seed an active local session with the given agent_cid.
    fn seed_active_session(conn: &mut SqliteConnection, agent_cid: &str) {
        crate::db::local_sessions::create_session(
            conn,
            crate::db::local_sessions::CreateLocalSessionInput {
                id: None,
                human_id: "human-1".into(),
                agent_pub_key: agent_cid.into(),
                doorway_url: "http://doorway.test".into(),
                doorway_id: None,
                identifier: "tester".into(),
                display_name: None,
                profile_image_hash: None,
                bootstrap_url: None,
            },
        )
        .expect("seed session");
    }

    /// The lockstep unfreeze: with NO cell-key snapshot at boot (`None`), a session
    /// that logs in AFTER boot is picked up per pass — so reconcile resolves the
    /// same agent_cid salvage authors, and the node's own rows are never orphaned.
    #[test]
    fn resolve_self_agent_cid_picks_up_post_boot_session() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        // Boot: no conductor cell key, no session → unresolved (transport-only).
        assert_eq!(resolve_self_agent_cid(&mut conn, None), None);

        // A session logs in after boot.
        seed_active_session(&mut conn, "uhCAk-session-self");
        assert_eq!(
            resolve_self_agent_cid(&mut conn, None).as_deref(),
            Some("uhCAk-session-self"),
            "post-boot session agent_cid is resolved without a restart"
        );
    }

    /// The cell-key snapshot is the stable fallback when no session is active, and
    /// a session (candidate a) wins over the cell key (candidate b) when present.
    #[test]
    fn resolve_self_agent_cid_prefers_session_then_falls_back_to_cellkey() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        // No session → the boot cell-key snapshot is used.
        assert_eq!(
            resolve_self_agent_cid(&mut conn, Some("uhCAk-cellkey")).as_deref(),
            Some("uhCAk-cellkey")
        );

        // Session present → it wins over the cell key (lockstep with salvage's
        // [session → cell key] order).
        seed_active_session(&mut conn, "uhCAk-session-self");
        assert_eq!(
            resolve_self_agent_cid(&mut conn, Some("uhCAk-cellkey")).as_deref(),
            Some("uhCAk-session-self")
        );
    }

    // -----------------------------------------------------------------------
    // T23: BlobStoreSnapshot adapter
    // -----------------------------------------------------------------------

    /// Snapshot taken after a blob is stored reports `has(hash) == true`,
    /// and reports `has` of an absent hash as `false`.
    #[tokio::test]
    async fn blob_store_snapshot_reflects_local_pantry() {
        let store = crate::blob_store::BlobStore::new_memory();
        let payload = b"reconcile-snapshot-fixture-v1";
        let result = store.store(payload).await.expect("store fixture blob");

        let snapshot = BlobStoreSnapshot::from_store(&store).expect("snapshot");

        assert!(
            snapshot.has(&result.cid),
            "snapshot must report the stored cid present"
        );
        assert!(
            !snapshot.has(&elohim_epr::address::compute_bytes_cid(b"never stored")),
            "snapshot must report an unknown cid absent"
        );
        assert_eq!(snapshot.len(), 1, "snapshot captured one hash");
        assert!(!snapshot.is_empty(), "snapshot is non-empty after store");
    }

    // -----------------------------------------------------------------------
    // P3-2: salvage_pass (salvage self-selection behind the placement seam)
    // -----------------------------------------------------------------------

    /// Strategy stub returning a fixed closest-set, so salvage tests control
    /// whether self is "closest" independently of the XOR internals (those are
    /// covered by `reconcile::placement` tests).
    struct StaticStrategy(Vec<String>);
    impl PlacementStrategy for StaticStrategy {
        fn rank(&self, _blob: &str, _candidates: &[PlacementCandidate], _n: usize) -> Vec<String> {
            self.0.clone()
        }
    }

    struct RecordingAuthor {
        authored: Mutex<Vec<(String, String, String)>>,
    }
    impl RecordingAuthor {
        fn new() -> Self {
            Self {
                authored: Mutex::new(Vec::new()),
            }
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

    fn salvage_cfg(enabled: bool, target: usize) -> SalvageConfig {
        SalvageConfig {
            enabled,
            target_replicas: target,
            inventory_freshness_seconds: 600,
        }
    }

    fn pool(cids: &[&str]) -> Vec<PlacementCandidate> {
        cids.iter()
            .map(|c| PlacementCandidate::from_agent_cid(*c))
            .collect()
    }

    /// Mark a provider as currently hosting a blob: a fresh inventory row under a
    /// bound witness (an admitted signed delta), the only kind a custody floor counts.
    fn mark_hosting(conn: &mut SqliteConnection, peer: &str, blob: &str, now: DateTime<Utc>) {
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let b = blob.to_string();
        apply_snapshot(conn, peer, std::slice::from_ref(&b), 1, &when).unwrap();
    }

    /// A fresh inventory row with no recorded witness — what every row written before
    /// the witness column reads as (`legacy-hint`).
    fn mark_legacy_hint(conn: &mut SqliteConnection, peer: &str, blob: &str, now: DateTime<Utc>) {
        diesel::insert_into(peer_blob_inventory::table)
            .values(&crate::db::models::NewPeerBlobInventoryRow {
                peer_id: peer.to_string(),
                blob_cid: blob.to_string(),
                last_seen_at: now.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                source: "gossip-delta".to_string(),
                sequence: 1,
            })
            .execute(conn)
            .unwrap();
    }

    /// A cross-signed `AgentPeerBinding` binds `test_agent(name)` to the libp2p
    /// PeerId of `secret`. Returns `(agent_cid, peer_id)`.
    fn bind_transport(
        conn: &mut SqliteConnection,
        name: &str,
        secret: [u8; 32],
    ) -> (String, String) {
        let peer = crate::test_util::bind_cross_signed(conn, name, secret);
        (crate::test_util::test_agent(name), peer)
    }

    fn run_salvage(
        conn: &mut SqliteConnection,
        now: DateTime<Utc>,
    ) -> (SalvageOutcome, RecordingAuthor) {
        let author = RecordingAuthor::new();
        let outcome = salvage_pass(
            conn,
            "self",
            "self",
            &StaticStrategy(vec!["self".into()]),
            &author,
            &pool(&["self"]),
            salvage_cfg(true, 2),
            now,
        )
        .unwrap();
        (outcome, author)
    }

    /// R2: two committed providers whose only evidence is fresh `legacy-hint` rows (no
    /// recorded witness) do not hold the floor: the blob is under-replicated and the
    /// closest volunteer adopts it. The rows stay fetch hints.
    #[test]
    fn custody_count_ignores_unbound_legacy_rows() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = cid_of('l');
        insert_custody_commitment(&mut conn, "c1", "prov-A", "steward-Z", &blob);
        insert_custody_commitment(&mut conn, "c2", "prov-B", "steward-Z", &blob);
        mark_legacy_hint(&mut conn, "prov-A", &blob, now);
        mark_legacy_hint(&mut conn, "prov-B", &blob, now);

        let (outcome, author) = run_salvage(&mut conn, now);
        assert_eq!(outcome.under_replicated, 1, "legacy rows hold no floor");
        assert_eq!(outcome.commitments_authored, 1);
        assert_eq!(author.authored.lock().unwrap().len(), 1);
        let stale_before = (now - chrono::Duration::seconds(600))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        assert_eq!(
            crate::db::peer_blob_inventory::lookup_hosts(&mut conn, &blob, &stale_before)
                .unwrap()
                .len(),
            2,
            "legacy rows remain fetch hints"
        );
    }

    /// R2: a bound row inside the freshness window counts; the same bound evidence
    /// past it does not. Evidence keyed by a provider's bound transport id counts for
    /// the provider's `agent_cid`.
    #[test]
    fn bound_fresh_row_counts_and_expired_one_does_not() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = cid_of('x');
        let (prov_a, peer_a) = bind_transport(&mut conn, "uhCAkProvA", [71u8; 32]);
        let (prov_b, peer_b) = bind_transport(&mut conn, "uhCAkProvB", [72u8; 32]);
        insert_custody_commitment(&mut conn, "c1", &prov_a, "steward-Z", &blob);
        insert_custody_commitment(&mut conn, "c2", &prov_b, "steward-Z", &blob);
        // A: fresh, bound, keyed by its transport id. B: bound, but an hour old.
        mark_hosting(&mut conn, &peer_a, &blob, now);
        mark_hosting(
            &mut conn,
            &peer_b,
            &blob,
            now - chrono::Duration::seconds(3600),
        );
        let (outcome, _) = run_salvage(&mut conn, now);
        assert_eq!(
            outcome.under_replicated, 1,
            "one fresh bound holder of two: the expired row does not count"
        );

        // B re-announces inside the window (the next in-order delta): the floor holds.
        apply_snapshot(
            &mut conn,
            &peer_b,
            std::slice::from_ref(&blob),
            2,
            &now.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        )
        .unwrap();
        let (outcome, author) = run_salvage(&mut conn, now);
        assert_eq!(outcome.under_replicated, 0, "two fresh bound holders");
        assert!(author.authored.lock().unwrap().is_empty());
    }

    /// The red-team inflation: a forged transport manifest naming a provider agent
    /// (`{iroh_node_id: forger, agent_cid: provider}`) must not make the forger's own
    /// bound inventory rows count as that provider's custody — nor may the floor
    /// read any manifest row at all. Only the provider's cross-signed transports count.
    #[test]
    fn custody_floor_ignores_rows_from_a_hijacked_transport() {
        use crate::db::diesel_schema::peer_transport_manifest::dsl as t;
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = cid_of('h');
        let (prov_a, peer_a) = bind_transport(&mut conn, "uhCAkProvA", [73u8; 32]);
        let prov_b = crate::test_util::test_agent("uhCAkProvB");
        insert_custody_commitment(&mut conn, "c1", &prov_a, "steward-Z", &blob);
        insert_custody_commitment(&mut conn, "c2", &prov_b, "steward-Z", &blob);
        mark_hosting(&mut conn, &peer_a, &blob, now);
        // The forger's own transports, hosting under bound witnesses (its signed
        // deltas are genuinely its own), hijack B's agent in the manifest table on
        // both columns.
        let forger_peer = crate::test_util::bind_cross_signed(&mut conn, "uhCAkForger", [74u8; 32]);
        let forger_node = hex::encode([75u8; 32]);
        mark_hosting(&mut conn, &forger_peer, &blob, now);
        mark_hosting(&mut conn, &forger_node, &blob, now);
        diesel::insert_into(t::peer_transport_manifest)
            .values((
                t::agent_cid.eq(&prov_b),
                t::libp2p_peer_id.eq(&forger_peer),
                t::iroh_node_id.eq(&forger_node),
                t::discovery_methods_json.eq("[]"),
                t::capability_level.eq(1),
                t::last_observed.eq(1_i64),
            ))
            .execute(&mut conn)
            .unwrap();

        let (outcome, author) = run_salvage(&mut conn, now);
        assert_eq!(
            outcome.under_replicated, 1,
            "B holds no floor: the hijacked transports speak for the forger, not for B"
        );
        assert_eq!(author.authored.lock().unwrap().len(), 1);
    }

    #[test]
    fn salvage_under_replicated_self_closest_authors() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = cid_of('b');
        // One existing custodian, target 2 → under-replicated.
        insert_custody_commitment(&mut conn, "c1", "prov-A", "steward-Z", &blob);
        mark_hosting(&mut conn, "prov-A", &blob, now);

        let author = RecordingAuthor::new();
        let strat = StaticStrategy(vec!["prov-A".into(), "self".into()]); // self is closest
        let outcome = salvage_pass(
            &mut conn,
            "self",
            "self",
            &strat,
            &author,
            &pool(&["self", "prov-A", "prov-B"]),
            salvage_cfg(true, 2),
            now,
        )
        .unwrap();

        assert_eq!(outcome.commitments_authored, 1);
        let recorded = author.authored.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded[0],
            (blob.clone(), "self".to_string(), "steward-Z".to_string()),
            "salvager authors provider=self, receiver=content steward"
        );
    }

    #[test]
    fn salvage_already_replicated_no_author() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = cid_of('c');
        insert_custody_commitment(&mut conn, "c1", "prov-A", "steward-Z", &blob);
        insert_custody_commitment(&mut conn, "c2", "prov-B", "steward-Z", &blob);
        // R2: "hosting" means BOUND, unexpired evidence — an admitted signed delta
        // from each provider (`mark_hosting`). Unwitnessed rows would not hold the
        // floor (`custody_count_ignores_unbound_legacy_rows`).
        mark_hosting(&mut conn, "prov-A", &blob, now);
        mark_hosting(&mut conn, "prov-B", &blob, now);

        let author = RecordingAuthor::new();
        let strat = StaticStrategy(vec!["self".into()]);
        let outcome = salvage_pass(
            &mut conn,
            "self",
            "self",
            &strat,
            &author,
            &pool(&["self"]),
            salvage_cfg(true, 2),
            now,
        )
        .unwrap();

        assert_eq!(outcome.commitments_authored, 0);
        assert_eq!(
            outcome.under_replicated, 0,
            "honored==target → not under-replicated"
        );
        assert!(author.authored.lock().unwrap().is_empty());
    }

    #[test]
    fn salvage_opted_out_no_author() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = cid_of('d');
        insert_custody_commitment(&mut conn, "c1", "prov-A", "steward-Z", &blob);
        mark_hosting(&mut conn, "prov-A", &blob, now);

        let author = RecordingAuthor::new();
        let strat = StaticStrategy(vec!["self".into()]); // would be closest, but opted out
        let outcome = salvage_pass(
            &mut conn,
            "self",
            "self",
            &strat,
            &author,
            &pool(&["self"]),
            salvage_cfg(false, 2),
            now,
        )
        .unwrap();

        assert_eq!(outcome.commitments_authored, 0);
        assert_eq!(outcome.under_replicated, 1);
        assert_eq!(outcome.skipped_opted_out, 1);
        assert!(author.authored.lock().unwrap().is_empty());
    }

    #[test]
    fn salvage_not_closest_no_author() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = cid_of('e');
        insert_custody_commitment(&mut conn, "c1", "prov-A", "steward-Z", &blob);
        mark_hosting(&mut conn, "prov-A", &blob, now);

        let author = RecordingAuthor::new();
        let strat = StaticStrategy(vec!["prov-A".into(), "prov-B".into()]); // self NOT closest
        let outcome = salvage_pass(
            &mut conn,
            "self",
            "self",
            &strat,
            &author,
            &pool(&["self", "prov-A", "prov-B"]),
            salvage_cfg(true, 2),
            now,
        )
        .unwrap();

        assert_eq!(outcome.commitments_authored, 0);
        assert_eq!(outcome.skipped_not_closest, 1);
        assert!(author.authored.lock().unwrap().is_empty());
    }

    #[test]
    fn salvage_already_provider_no_author() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = cid_of('f');
        // self is already a provider (not yet hosting → honored 0 → under-replicated),
        // so it must NOT author a duplicate adopt.
        insert_custody_commitment(&mut conn, "c1", "self", "steward-Z", &blob);

        let author = RecordingAuthor::new();
        let strat = StaticStrategy(vec!["self".into()]);
        let outcome = salvage_pass(
            &mut conn,
            "self",
            "self",
            &strat,
            &author,
            &pool(&["self"]),
            salvage_cfg(true, 2),
            now,
        )
        .unwrap();

        assert_eq!(
            outcome.commitments_authored, 0,
            "already a provider — no duplicate adopt"
        );
        assert!(author.authored.lock().unwrap().is_empty());
    }

    /// The write/read split: when self's transport id (`self_read`) differs from
    /// its agent_cid (`self_write`), the authored `provider` is the AGENT_CID, and
    /// the candidate-pool / rank identity is the agent_cid too. The transport id is
    /// NEVER written.
    #[test]
    fn salvage_authors_agent_cid_not_transport_when_they_differ() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = cid_of('a');
        insert_custody_commitment(&mut conn, "c1", "prov-A", "steward-Z", &blob);
        mark_hosting(&mut conn, "prov-A", &blob, now);

        let author = RecordingAuthor::new();
        // Strategy names the agent_cid as closest (the pool identity).
        let strat = StaticStrategy(vec!["prov-A".into(), "uhCAk-self".into()]);
        let outcome = salvage_pass(
            &mut conn,
            "12D3KooTransportSelf", // self_read (transport)
            "uhCAk-self",           // self_write (agent_cid)
            &strat,
            &author,
            &pool(&["uhCAk-self", "prov-A"]),
            salvage_cfg(true, 2),
            now,
        )
        .unwrap();

        assert_eq!(outcome.commitments_authored, 1);
        let recorded = author.authored.lock().unwrap();
        assert_eq!(
            recorded[0],
            (blob, "uhCAk-self".to_string(), "steward-Z".to_string()),
            "provider is the agent_cid, never the transport id"
        );
    }

    /// Backward-compat idempotency: a LEGACY row whose `provider` is the transport
    /// `self_read` id must still count as "already a provider" (no duplicate adopt)
    /// even though the node now writes agent_cid.
    #[test]
    fn salvage_already_provider_matches_legacy_transport_row() {
        let p = test_pool();
        let mut conn = p.get().unwrap();
        let now = chrono::Utc::now();
        let blob = cid_of('a');
        // Legacy row: provider is the transport id this node used to write.
        insert_custody_commitment(&mut conn, "c1", "12D3KooTransportSelf", "steward-Z", &blob);

        let author = RecordingAuthor::new();
        let strat = StaticStrategy(vec!["uhCAk-self".into()]);
        let outcome = salvage_pass(
            &mut conn,
            "12D3KooTransportSelf", // self_read matches the legacy provider
            "uhCAk-self",           // self_write (agent_cid)
            &strat,
            &author,
            &pool(&["uhCAk-self"]),
            salvage_cfg(true, 2),
            now,
        )
        .unwrap();

        assert_eq!(
            outcome.commitments_authored, 0,
            "legacy transport-id provider row counts as already-provider (no duplicate)"
        );
        assert!(author.authored.lock().unwrap().is_empty());
    }

    /// REGRESSION (alpha 2026-07-31, both doorways): a node that re-keys must
    /// keep claiming bytes it still holds.
    ///
    /// Before the re-claim arm, the second pass filtered the already-manifested
    /// blob out as `skipped_existing` and recorded nothing, so `shard_locations`
    /// held ONLY the fossil-keyed row. The resilience holder join
    /// (`shard_locations.peer_id == humans.agent_pub_key`) then matched no live
    /// human and the card read `stewardingCollectives: 0` while
    /// `GET /blob/{hash}` served the object in full — exactly the live symptom.
    #[tokio::test]
    async fn reclaims_self_held_custody_under_a_new_agent_key() {
        const OLD_KEY: &str = "uhCAkFossilAgentKeyFromABeforeTimesReinstall";
        const NEW_KEY: &str = "uhCAkLiveAgentKeyMintedByTheLatestReinstall";

        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let store = BlobStore::new_memory();

        // Small, unstriped payload: its single chunk IS the payload.
        let stored = store.store(b"the landing page bytes").await.expect("store");
        let hash = stored.cid.to_string();

        // Pass 1 — under the OLD identity: manifests the blob and claims it.
        let first = manifest_backfill_pass(&mut conn, &store, Some(OLD_KEY), "lamad", 16)
            .await
            .expect("first pass");
        assert_eq!(first.manifests_written, 1, "blob is manifested once");
        assert_eq!(first.self_held_recorded, 1, "and claimed under the old key");
        assert_eq!(first.self_held_reclaimed, 0, "nothing to re-claim yet");

        // Pass 2 — the pod has been re-keyed. The manifest already exists, so the
        // blob is `skipped_existing`; the re-claim arm must still fire.
        let second = manifest_backfill_pass(&mut conn, &store, Some(NEW_KEY), "lamad", 16)
            .await
            .expect("second pass");
        assert_eq!(second.manifests_written, 0, "manifest is not rewritten");
        assert_eq!(second.skipped_existing, 1, "the blob is a pre-filter hit");
        assert_eq!(
            second.self_held_reclaimed, 1,
            "custody must be re-asserted under the live key — this is the fix"
        );

        let holders: std::collections::HashSet<String> =
            crate::db::shard_locations::get_locations_for_shard(&mut conn, &hash)
                .expect("load locations")
                .into_iter()
                .map(|l| l.peer_id)
                .collect();
        assert!(
            holders.contains(NEW_KEY),
            "the live key must be a recorded holder, else the resilience join \
             stays empty; got {holders:?}"
        );
        assert!(
            holders.contains(OLD_KEY),
            "the fossil row is left inert, not deleted (rekey cascade is \
             membership_identity_reconcile's job); got {holders:?}"
        );

        // Pass 3 — steady state under the same key writes nothing new (upsert is
        // keyed `(shard_hash, peer_id)`), so the arm cannot grow the table.
        let third = manifest_backfill_pass(&mut conn, &store, Some(NEW_KEY), "lamad", 16)
            .await
            .expect("third pass");
        assert_eq!(
            third.self_held_reclaimed, 0,
            "an already-current claim is not re-recorded"
        );
        assert_eq!(
            crate::db::shard_locations::get_locations_for_shard(&mut conn, &hash)
                .expect("load locations")
                .len(),
            2,
            "exactly the fossil row + the live row"
        );
    }

    /// The re-claim arm inherits `record_self_held_shard`'s namespace refusal: a
    /// transport id must never reach the `agent_cid` join-key column.
    #[tokio::test]
    async fn reclaim_refuses_a_transport_identity() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let store = BlobStore::new_memory();
        let stored = store
            .store(b"bytes held by a node with no agent key")
            .await
            .expect("store");

        manifest_backfill_pass(&mut conn, &store, Some("uhCAkRealAgentKey"), "lamad", 16)
            .await
            .expect("first pass");

        let second = manifest_backfill_pass(
            &mut conn,
            &store,
            Some("12D3KooWTransportIdNotAnAgentCid"),
            "lamad",
            16,
        )
        .await
        .expect("second pass");
        assert_eq!(
            second.self_held_reclaimed, 0,
            "a libp2p id can never join humans.agent_pub_key; it must not be written"
        );

        let holders: Vec<String> =
            crate::db::shard_locations::get_locations_for_shard(&mut conn, &stored.cid.to_string())
                .expect("load locations")
                .into_iter()
                .map(|l| l.peer_id)
                .collect();
        assert_eq!(holders, vec!["uhCAkRealAgentKey".to_string()]);
    }

    const UNIT_AGENT: &str = "uhCAkUnitSweepUploaderAgentKey";

    fn self_rows(conn: &mut SqliteConnection, unit: &ByteCid) -> Vec<String> {
        crate::db::shard_locations::get_locations_for_shard(conn, &unit.to_string())
            .expect("load locations")
            .into_iter()
            .filter(|l| l.peer_id == UNIT_AGENT)
            .map(|l| l.status)
            .collect()
    }

    /// A payload cut into several chunks (under the striping threshold): the backfill
    /// rows every chunk the store serves, not only the payload.
    #[tokio::test]
    async fn multi_chunk_payload_rows_every_chunk() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let store = BlobStore::new_memory();
        let stored = store
            .store(&crate::blob_store::tests::xorshift_bytes(6 * 1024 * 1024))
            .await
            .expect("store");
        assert!(stored.chunks.len() > 1, "fixture must be chunked");

        let outcome = manifest_backfill_pass(&mut conn, &store, Some(UNIT_AGENT), "lamad", 64)
            .await
            .expect("pass");
        assert_eq!(outcome.errors, 0);
        for chunk in &stored.chunks {
            assert_eq!(
                self_rows(&mut conn, &chunk.cid),
                vec!["self-held".to_string()],
                "chunk {} has its own self-held row",
                chunk.cid
            );
        }
        assert!(
            self_rows(&mut conn, &stored.cid).is_empty(),
            "the marker names a payload, not a unit: no row for it"
        );

        // Steady state: nothing new to record.
        let again = manifest_backfill_pass(&mut conn, &store, Some(UNIT_AGENT), "lamad", 64)
            .await
            .expect("second pass");
        assert_eq!(again.units_recorded, 0);
    }

    /// An upload whose manifest is erasure-coded: the per-payload arm never self-claims
    /// a striped manifest's derived shards, so the unit sweep is what rows the chunks
    /// the node serves and any shard another node pushed here.
    #[tokio::test]
    async fn striped_upload_rows_cover_every_served_unit() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let store = BlobStore::new_memory();
        let data = crate::blob_store::tests::xorshift_bytes(3 * 1024 * 1024);
        let stored = store.store(&data).await.expect("store");

        // The upload's manifest is striped (threshold lowered so the fixture stays small).
        let encoder = crate::sharding::ShardEncoder::new(crate::sharding::ShardConfig {
            rs_threshold: 0,
            ..crate::sharding::ShardConfig::default()
        });
        let (manifest, shards) = encoder
            .encode(&data, "application/octet-stream", "commons")
            .expect("encode");
        assert!(!manifest.stripes.is_empty(), "fixture must be striped");
        crate::db::shard_manifests::record_generated_manifest(
            &mut conn,
            &format!("blob:{}", stored.cid),
            "lamad",
            &manifest,
        )
        .expect("manifest");
        // One shard another node pushed here, held under its own CID.
        let pushed = store.store(&shards[0][0]).await.expect("pushed shard");
        assert_eq!(pushed.chunks.len(), 1, "a small shard is one inline unit");

        let outcome = manifest_backfill_pass(&mut conn, &store, Some(UNIT_AGENT), "lamad", 64)
            .await
            .expect("pass");
        assert_eq!(outcome.errors, 0);

        let served = store.list_units().expect("list_units");
        assert!(served.contains(&pushed.cid), "the pushed shard is a unit");
        for chunk in &stored.chunks {
            assert!(served.contains(&chunk.cid), "every chunk is a unit");
        }
        for unit in &served {
            assert_eq!(
                self_rows(&mut conn, unit),
                vec!["self-held".to_string()],
                "served unit {unit} has a self-held row"
            );
        }
    }

    /// A unit whose bytes do not hash to its name is quarantined by the sweep and never
    /// rowed, however the name looks on disk.
    #[tokio::test]
    async fn unit_sweep_rows_nothing_it_cannot_read_back_verified() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let store = BlobStore::new_memory();
        let stored = store
            .store(&crate::blob_store::tests::xorshift_bytes(6 * 1024 * 1024))
            .await
            .expect("store");
        let victim = stored.chunks[1].cid;
        let path = {
            let root = store.root_dir().join("chunks");
            let name = victim.to_string();
            root.join(name.get(7..11).unwrap()).join(&name)
        };
        let mut bytes = std::fs::read(&path).expect("chunk file");
        bytes[3] ^= 0xff;
        std::fs::write(&path, bytes).expect("corrupt");

        manifest_backfill_pass(&mut conn, &store, Some(UNIT_AGENT), "lamad", 64)
            .await
            .expect("pass");
        assert!(self_rows(&mut conn, &victim).is_empty());
        assert!(!store.list_units().unwrap().contains(&victim));
    }

    /// A local copy that is named but corrupt is not a held copy: the pass kicks a
    /// re-fetch for it, and stops kicking once the copy is whole again.
    #[tokio::test]
    async fn reconcile_refetches_a_corrupt_local_copy() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let store = BlobStore::new_memory();
        let stored = store
            .store(&crate::blob_store::tests::xorshift_bytes(6 * 1024 * 1024))
            .await
            .expect("store");
        let blob_cid = stored.cid.to_string();
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &blob_cid);
        let when = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_cid),
            1,
            &when,
        )
        .unwrap();

        let run = |conn: &mut SqliteConnection| {
            let kicker = RecordingKicker {
                kicks: Mutex::new(Vec::new()),
            };
            reconcile_pass(
                conn,
                "self_cid",
                None,
                &VerifiedPantry::new(&store),
                &kicker,
                default_cfg(),
                chrono::Utc::now(),
                &[],
            )
            .unwrap();
            let kicks = kicker.kicks.lock().unwrap().len();
            kicks
        };
        assert_eq!(run(&mut conn), 0, "a whole local copy needs no fetch");

        let victim = stored.chunks[1].cid.to_string();
        let path = {
            let root = store.root_dir().join("chunks");
            root.join(victim.get(7..11).unwrap()).join(&victim)
        };
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[2] ^= 0xff;
        std::fs::write(&path, bytes).unwrap();
        // The filename snapshot still says "held"; the verified pantry does not.
        assert!(BlobStoreSnapshot::from_store(&store)
            .unwrap()
            .has(&stored.cid));
        assert_eq!(run(&mut conn), 1, "a corrupt local copy is re-fetched");
    }
}
